use anyhow::Result;
use ash::vk;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::context::EnkiEngine;
use crate::nam_args_api::{AccessIntent, AccessMode, ArgDescriptor, MemoryDomain};
use crate::pipeline_synthesis::cache::{
    DiskCacheManager, calculate_compute_key, calculate_source_hash,
};
use crate::pipeline_synthesis::compiler::ComputePipelineFactory;
use crate::pipeline_synthesis::types::{ComputeSynthesisInput, TaskCompilationArtifact};
use parsu::compiler::{ParsuArgDescriptorC, ParsuCompiler};

fn find_project_root() -> Option<PathBuf> {
    let mut curr = std::env::current_dir().ok()?;
    loop {
        if curr.join("Cargo.toml").exists() {
            return Some(curr);
        }
        if !curr.pop() {
            break;
        }
    }
    None
}

fn prompt_interactive_config() -> bool {
    if !io::stdin().is_terminal() {
        return false;
    }

    eprintln!(
        "\n\x1b[1;33mwarning\x1b[0m\x1b[1m: missing required compilation profile and LLVM bitcode for GPU JIT synthesis\x1b[0m\n\
         \x1b[1;34m  =\x1b[0m \x1b[1mnote:\x1b[0m GPU synthesis requires optimization (`opt-level = 2`) to lower host abstractions into silicon primitives\n\
         \x1b[1;34m  =\x1b[0m \x1b[1mnote:\x1b[0m execution requires `target.'cfg(all())'.rustflags = [\"--emit=llvm-bc\"]`\n\
         \x1b[1;34m  =\x1b[0m \x1b[1mhelp:\x1b[0m configuration can be appended to `.cargo/config.toml` safely without modifying existing blocks"
    );

    eprint!(
        "\x1b[1;34m  -->\x1b[0m \x1b[1mappend configuration to `.cargo/config.toml`? [Y/n]\x1b[0m "
    );
    let _ = io::stderr().flush();

    let mut answer = String::new();
    if io::stdin().read_line(&mut answer).is_ok() {
        let trimmed = answer.trim();
        trimmed.is_empty() || trimmed.eq_ignore_ascii_case("y")
    } else {
        false
    }
}

fn apply_config_and_restart() {
    let root = match find_project_root() {
        Some(r) => r,
        None => return,
    };

    let cargo_dir = root.join(".cargo");
    let config_path = cargo_dir.join("config.toml");

    let _ = fs::create_dir_all(&cargo_dir);

    let already_configured = fs::read_to_string(&config_path)
        .map(|c| c.contains("--emit=llvm-bc"))
        .unwrap_or(false);

    if !already_configured {
        let snippet = r#"
# --- Added by Enki for GPU JIT compilation ---
[profile.dev]
opt-level = 2
codegen-units = 1

[profile.dev.package."*"]
opt-level = 2

[profile.release]
opt-level = 2
codegen-units = 1

[profile.release.package."*"]
opt-level = 2

[target.'cfg(all())']
rustflags = ["--emit=llvm-bc"]
# ---------------------------------------------
"#;

        let append_res = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&config_path)
            .and_then(|mut f| f.write_all(snippet.as_bytes()));

        if append_res.is_err() {
            return;
        }
    }

    eprintln!("\x1b[1;32m     Updated\x1b[0m `.cargo/config.toml` successfully");
    eprintln!("\x1b[1;32m   Re-running\x1b[0m `cargo run` with LLVM bitcode generation...\n");

    let mut cmd = Command::new("cargo");
    cmd.arg("run");

    if !cfg!(debug_assertions) {
        cmd.arg("--release");
    }

    let user_args: Vec<String> = std::env::args().skip(1).collect();
    if !user_args.is_empty() {
        cmd.arg("--");
        cmd.args(&user_args);
    }

    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    match cmd.status() {
        Ok(status) => std::process::exit(status.code().unwrap_or(1)),
        Err(err) => {
            eprintln!("\x1b[1;31merror\x1b[0m: failed to re-execute `cargo run`: {err}");
            std::process::exit(1);
        }
    }
}

fn convert_to_parsu_c_descriptors(descriptors: &[ArgDescriptor]) -> Vec<ParsuArgDescriptorC> {
    descriptors
        .iter()
        .map(|d| {
            let mode_val = match d.mode {
                AccessMode::ByValue => 0,
                AccessMode::SpmdCellMut => 1,
                AccessMode::SpmdCellConst => 2,
                AccessMode::GlobalSliceRead => 3,
                AccessMode::GlobalSliceReadWrite => 4,
                AccessMode::AtomicCell => 5,
                AccessMode::AtomicSlice => 6,
                AccessMode::WorkgroupScratchpad => 7,
            };

            let domain_val = match d.domain {
                MemoryDomain::ArenaPayload => 0,
                MemoryDomain::PhysicalStorageBuffer => 1,
                MemoryDomain::WorkgroupShared => 2,
                MemoryDomain::ZeroSized => 3,
            };

            let intent_val = match d.intent {
                AccessIntent::Read => 0,
                AccessIntent::Write => 1,
                AccessIntent::ReadWrite => 2,
            };

            ParsuArgDescriptorC {
                mode: mode_val,
                domain: domain_val,
                intent: intent_val,
                is_optional: if d.is_optional { 1 } else { 0 },
                element_size: d.element_size,
                stride: d.stride,
                alignment: d.alignment,
                arena_size_bytes: d.arena_size_bytes,
            }
        })
        .collect()
}

fn find_native_bitcode() -> Result<PathBuf> {
    let exe_path = env::current_exe()?;
    let raw_name = exe_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::anyhow!("Failed to parse executable name"))?;

    let exe_name = raw_name.strip_suffix(".exe").unwrap_or(raw_name);
    let exe_name_alt = exe_name.replace('-', "_");

    let exe_dir = exe_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Failed to find executable directory"))?;

    let deps_dir = exe_dir.join("deps");
    if !deps_dir.exists() {
        return Err(anyhow::anyhow!(
            "Dependencies directory not found at {:?}",
            deps_dir
        ));
    }

    let mut newest_bc: Option<(PathBuf, std::time::SystemTime)> = None;

    if let Ok(entries) = fs::read_dir(deps_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "bc") {
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                let matches_exe =
                    file_name.starts_with(exe_name) || file_name.starts_with(&exe_name_alt);

                if matches_exe && let Ok(mtime) = entry.metadata().and_then(|m| m.modified()) {
                    newest_bc = Some(newest_bc.map_or(
                        (path.clone(), mtime),
                        |(ref best_path, best_mtime)| {
                            if mtime > best_mtime {
                                (path, mtime)
                            } else {
                                (best_path.clone(), best_mtime)
                            }
                        },
                    ));
                }
            }
        }
    }

    newest_bc.map(|(path, _)| path).ok_or_else(|| {
        anyhow::anyhow!(
            "Bitcode file not found for '{}'. Please rebuild your project using 'cargo build'.",
            exe_name
        )
    })
}

pub struct JitCoordinator;

impl JitCoordinator {
    pub fn orchestrate(engine: &EnkiEngine, input: &ComputeSynthesisInput) -> Result<()> {
        let cache_manager = DiskCacheManager::new();
        let target_nam_name = &input.nam_name;
        let local_size = input.local_size;

        let bc_path = match find_native_bitcode() {
            Ok(path) => path,
            Err(_) => {
                if prompt_interactive_config() {
                    apply_config_and_restart();
                }

                let diag = crate::diagnostics::ingress::compiler::bitcode_not_found();
                let formatted = crate::diagnostics::emit_diagnostic(&diag);
                return Err(anyhow::anyhow!("{formatted}"));
            }
        };

        let bc_mtime = fs::metadata(&bc_path).and_then(|m| m.modified()).ok();

        let hash_key = calculate_compute_key(
            calculate_source_hash(target_nam_name),
            target_nam_name,
            local_size,
            &input.arg_descriptors,
        );

        let cache_path = cache_manager.get_cache_path(hash_key);
        let cache_mtime = fs::metadata(&cache_path).and_then(|m| m.modified()).ok();

        let is_dirty = match (bc_mtime, cache_mtime) {
            (Some(bc_time), Some(cache_time)) => bc_time > cache_time,
            _ => true,
        };

        let mut cache_lock = engine.pipeline_cache.lock().unwrap();

        if is_dirty {
            let bc_path_str = bc_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid bitcode path string"))?;

            let project_root = input.host_manifest_dir.clone().unwrap_or_else(|| {
                std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| {
                    std::env::current_dir()
                        .unwrap()
                        .to_string_lossy()
                        .to_string()
                })
            });

            let local_size_i32 = (
                local_size.0 as i32,
                local_size.1 as i32,
                local_size.2 as i32,
            );

            let c_descriptors = convert_to_parsu_c_descriptors(&input.arg_descriptors);

            let compiled_nams = match ParsuCompiler::compile_bitcode(
                bc_path_str,
                target_nam_name,
                &project_root,
                local_size_i32,
                &c_descriptors,
            ) {
                Ok(nams) => nams,
                Err(parsu::compiler::ParsuError::ToolchainMissing) => {
                    let diag = crate::diagnostics::ingress::compiler::toolchain_not_found();
                    let formatted = crate::diagnostics::emit_diagnostic(&diag);
                    return Err(anyhow::anyhow!("{formatted}"));
                }

                Err(parsu::compiler::ParsuError::Diagnostics(diags)) => {
                    let unified_diags = crate::diagnostics::from_parsu_batch(diags);

                    let error_count = unified_diags.len();
                    let formatted = crate::diagnostics::emit_batch(&unified_diags);

                    let bin_name = std::env::current_exe()
                        .ok()
                        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                        .unwrap_or_else(|| "enki_app".to_string());

                    let count_str = if error_count == 1 {
                        "1 previous error".to_string()
                    } else {
                        format!("{error_count} previous errors")
                    };

                    let summary = format!(
                        "\x1b[1;91merror\x1b[0m: could not compile `{bin_name}` (nam `{target_nam_name}`) due to {count_str}\n"
                    );

                    return Err(anyhow::anyhow!("{formatted}{summary}"));
                }
                Err(parsu::compiler::ParsuError::Generic(msg)) => {
                    return Err(anyhow::anyhow!("Parsu compilation failed: {}", msg));
                }
            };

            let target_nam = compiled_nams
                .into_iter()
                .find(|k| target_nam_name.ends_with(&k.nam_name))
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "[JitCoordinator] Nam '{}' not found in compiled modules",
                        target_nam_name
                    )
                })?;

            let stack_size_per_thread = target_nam.stack_size_per_thread;

            let spirv_bytes = bytemuck::cast_slice::<u32, u8>(&target_nam.spirv_bytecode);

            let (pipeline, layout) =
                ComputePipelineFactory::compile(engine, spirv_bytes, *cache_lock)?;

            let artifact = TaskCompilationArtifact {
                pipeline,
                layout,
                local_size,
                stack_size_per_thread,
            };

            engine.synthesizer.cache.insert(hash_key, artifact);

            let pipeline_cache_data = unsafe {
                engine
                    .raw_device()
                    .get_pipeline_cache_data(*cache_lock)
                    .unwrap_or_default()
            };

            let _ = cache_manager.save_cache(
                hash_key,
                &engine.hardware_profile,
                &pipeline_cache_data,
                spirv_bytes,
                stack_size_per_thread,
            );
        } else {
            if let Some((pipeline_cache_data, spirv_bytes, stack_size_per_thread)) =
                cache_manager.load_cache(hash_key, &engine.hardware_profile)
            {
                device_update_pipeline_cache(
                    engine.raw_device(),
                    &mut cache_lock,
                    &pipeline_cache_data,
                );

                if let Ok((pipeline, layout)) =
                    ComputePipelineFactory::compile(engine, &spirv_bytes, *cache_lock)
                {
                    let artifact = TaskCompilationArtifact {
                        pipeline,
                        layout,
                        local_size,
                        stack_size_per_thread,
                    };
                    engine.synthesizer.cache.insert(hash_key, artifact);
                }
            }
        }

        Ok(())
    }
}

fn device_update_pipeline_cache(
    device: &ash::Device,
    cache_lock: &mut vk::PipelineCache,
    pipeline_cache_data: &[u8],
) {
    unsafe {
        device.destroy_pipeline_cache(*cache_lock, None);
        let create_info = vk::PipelineCacheCreateInfo::default().initial_data(pipeline_cache_data);
        if let Ok(new_cache) = device.create_pipeline_cache(&create_info, None) {
            *cache_lock = new_cache;
        }
    }
}
