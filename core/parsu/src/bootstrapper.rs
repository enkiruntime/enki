use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use tar::Archive;

pub const PARSU_VERSION: &str = "0.1.0";

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub const TARGET_TRIPLE: &str = "x86_64-unknown-linux-gnu";
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub const LIB_FILENAME: &str = "libparsu_compiler.so";
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub const ARCHIVE_EXTENSION: &str = "tar.gz";

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub const TARGET_TRIPLE: &str = "x86_64-pc-windows-msvc";
#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub const LIB_FILENAME: &str = "parsu_compiler.dll";
#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub const ARCHIVE_EXTENSION: &str = "tar.gz";

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub const TARGET_TRIPLE: &str = "aarch64-apple-darwin";
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub const LIB_FILENAME: &str = "libparsu_compiler.dylib";
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub const ARCHIVE_EXTENSION: &str = "tar.gz";

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub const EXPECTED_SHA256: &str =
    "766b53915e124681bf2d18a3cd0282de3b72ceefd57114dc795fb5f4b7139be8";

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub const EXPECTED_SHA256: &str =
    "81a185cfb047ac2db94c9f103210f3c3fa9f23f5805cb8e6c681c04216c11bad";

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub const EXPECTED_SHA256: &str = "";

const MASTER_PIN_HASH: &str = "218e19b44b9de9e57aad8f3520a0605838eb58af2fa1ac49a4a080acf35ee421";
const SALT: &str = "ENKI_DEV_SECRET_SALT_2026";

pub fn is_developer_authorized() -> bool {
    let dev_key = match std::env::var("PARSU_DEV_KEY") {
        Ok(k) if !k.trim().is_empty() => k,
        _ => return false,
    };

    let mut hasher = Sha256::new();
    hasher.update(dev_key.trim().as_bytes());
    hasher.update(SALT.as_bytes());
    let computed_hash = format!("{:x}", hasher.finalize());

    computed_hash == MASTER_PIN_HASH
}

fn find_compiler_source_dir() -> Option<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let direct_candidate = manifest_dir.join("src").join("compiler");
    if direct_candidate.join("CMakeLists.txt").exists() {
        return Some(direct_candidate);
    }

    if let Ok(mut curr) = std::env::current_dir() {
        loop {
            let candidate_workspace = curr.join("core").join("parsu").join("src").join("compiler");
            if candidate_workspace.join("CMakeLists.txt").exists() {
                return Some(candidate_workspace);
            }
            let candidate_simple = curr.join("src").join("compiler");
            if candidate_simple.join("CMakeLists.txt").exists() {
                return Some(candidate_simple);
            }
            if !curr.pop() {
                break;
            }
        }
    }

    None
}

fn detect_c_compiler() -> Option<PathBuf> {
    if let Ok(c) = std::env::var("CC") {
        let p = PathBuf::from(c);
        if p.exists() {
            return Some(p);
        }
    }
    let candidates = [
        "/usr/bin/clang-22",
        "/usr/lib/llvm-22/bin/clang",
        "/usr/bin/clang",
        "/usr/bin/gcc",
    ];
    candidates.iter().map(PathBuf::from).find(|p| p.exists())
}

fn detect_cxx_compiler() -> Option<PathBuf> {
    if let Ok(cxx) = std::env::var("CXX") {
        let p = PathBuf::from(cxx);
        if p.exists() {
            return Some(p);
        }
    }
    let candidates = [
        "/usr/bin/clang++-22",
        "/usr/lib/llvm-22/bin/clang++",
        "/usr/bin/clang++",
        "/usr/bin/g++",
    ];
    candidates.iter().map(PathBuf::from).find(|p| p.exists())
}

pub fn build_local_parsu() -> Result<PathBuf, String> {
    let source_dir = find_compiler_source_dir().ok_or_else(|| {
        "Failed to locate 'src/compiler/CMakeLists.txt' in local repository.".to_string()
    })?;

    let build_dir = source_dir.join("build_dev");
    let ninja_file = build_dir.join("build.ninja");

    let start = Instant::now();

    if !ninja_file.exists() {
        if build_dir.exists() {
            let _ = fs::remove_dir_all(&build_dir);
        }

        eprintln!(
            "\x1b[32;1m{:>12}\x1b[0m local Parsu C++ CMake project (one-time setup)...",
            "Configuring"
        );

        let mut config_cmd = Command::new("cmake");
        config_cmd.arg("-B").arg(&build_dir);
        config_cmd.arg("-G").arg("Ninja");
        config_cmd.arg("-DCMAKE_BUILD_TYPE=Release");

        if let Ok(llvm_dir) = std::env::var("LLVM_DIR") {
            config_cmd.arg(format!("-DLLVM_DIR={llvm_dir}"));
        } else if Path::new("/usr/lib/llvm-22/lib/cmake/llvm").exists() {
            config_cmd.arg("-DLLVM_DIR=/usr/lib/llvm-22/lib/cmake/llvm");
        }

        if let Ok(mlir_dir) = std::env::var("MLIR_DIR") {
            config_cmd.arg(format!("-DMLIR_DIR={mlir_dir}"));
        } else if Path::new("/usr/lib/llvm-22/lib/cmake/mlir").exists() {
            config_cmd.arg("-DMLIR_DIR=/usr/lib/llvm-22/lib/cmake/mlir");
        }

        if let Some(c_compiler) = detect_c_compiler() {
            config_cmd.arg(format!("-DCMAKE_C_COMPILER={}", c_compiler.display()));
        }
        if let Some(cxx_compiler) = detect_cxx_compiler() {
            config_cmd.arg(format!("-DCMAKE_CXX_COMPILER={}", cxx_compiler.display()));
        }

        config_cmd.arg(&source_dir);

        let status = config_cmd
            .status()
            .map_err(|e| format!("Failed to execute 'cmake': {}", e))?;

        if !status.success() {
            let _ = fs::remove_dir_all(&build_dir);
            return Err("CMake configuration failed for local Parsu build.".to_string());
        }
    }

    let mut build_cmd = Command::new("cmake");
    build_cmd.arg("--build").arg(&build_dir);
    build_cmd.arg("--parallel");

    let output = build_cmd
        .output()
        .map_err(|e| format!("Failed to execute 'cmake --build': {}", e))?;

    if !output.status.success() {
        let err_log = String::from_utf8_lossy(&output.stderr);
        let out_log = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "Local Parsu C++ build failed!\nSTDOUT:\n{}\nSTDERR:\n{}",
            out_log, err_log
        ));
    }

    let lib_candidates = [
        build_dir.join(LIB_FILENAME),
        build_dir.join("Release").join(LIB_FILENAME),
    ];

    for candidate in lib_candidates {
        if candidate.exists() {
            let duration = start.elapsed().as_secs_f32();
            eprintln!(
                "\x1b[1;92m{:>12}\x1b[0m local Parsu toolchain in \x1b[1m{:.2}s\x1b[0m (\x1b[2m{}\x1b[0m)\n",
                "Compiled",
                duration,
                candidate.display()
            );
            return Ok(candidate);
        }
    }

    Err(format!(
        "Build succeeded but output library '{}' was not found in '{}'",
        LIB_FILENAME,
        build_dir.display()
    ))
}

pub fn get_toolchain_dir() -> PathBuf {
    let cargo_home = std::env::var("CARGO_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".cargo")
        });

    cargo_home
        .join("enki")
        .join("toolchains")
        .join(format!("v{}", PARSU_VERSION))
        .join(TARGET_TRIPLE)
}

pub fn get_toolchain_lib_path() -> PathBuf {
    get_toolchain_dir().join(LIB_FILENAME)
}

pub fn ensure_parsu_ready() -> Result<PathBuf, String> {
    if is_developer_authorized() {
        return build_local_parsu();
    }

    if let Ok(dev_path) = std::env::var("PARSU_LIB_PATH") {
        let p = PathBuf::from(dev_path);
        if p.exists() {
            return Ok(p);
        }
        return Err(format!(
            "PARSU_LIB_PATH was set but file does not exist: {:?}",
            p
        ));
    }

    let target_lib = get_toolchain_lib_path();
    if target_lib.exists() {
        return Ok(target_lib);
    }

    eprintln!(
        "\n\x1b[1;33mwarning\x1b[0m\x1b[1m: Enki GPU JIT backend (`parsu`) is not installed on this machine\x1b[0m\n\
         \x1b[1;34m  =\x1b[0m \x1b[1mnote:\x1b[0m target: \x1b[1m{}\x1b[0m (toolchain v{})\n\
         \x1b[1;34m  =\x1b[0m \x1b[1mnote:\x1b[0m destination: \x1b[2m{}\x1b[0m",
        TARGET_TRIPLE,
        PARSU_VERSION,
        target_lib.display()
    );

    let should_download = if std::env::var("ENKI_AUTO_DOWNLOAD").is_ok() {
        true
    } else if io::stdin().is_terminal() {
        eprint!(
            "\x1b[1;34m  -->\x1b[0m \x1b[1mdownload and configure `parsu` toolchain automatically? [Y/n]\x1b[0m "
        );
        let _ = io::stderr().flush();
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_ok() {
            let trimmed = input.trim();
            trimmed.is_empty() || trimmed.eq_ignore_ascii_case("y")
        } else {
            false
        }
    } else {
        return Err(
            "Parsu backend is missing and terminal is non-interactive. Set `ENKI_AUTO_DOWNLOAD=1` to auto-install."
                .to_string(),
        );
    };

    if !should_download {
        return Err(
            "Operation aborted by user. GPU JIT execution cannot proceed without Parsu."
                .to_string(),
        );
    }

    let toolchain_dir = get_toolchain_dir();
    download_and_unpack_toolchain(&toolchain_dir)?;

    Ok(target_lib)
}

fn download_and_unpack_toolchain(dest_dir: &Path) -> Result<(), String> {
    let download_url = std::env::var("ENKI_PARSU_URL").unwrap_or_else(|_| {
        format!(
            "https://github.com/enkiruntime/enki/releases/download/v{}/parsu-v{}-{}.{}",
            PARSU_VERSION, PARSU_VERSION, TARGET_TRIPLE, ARCHIVE_EXTENSION
        )
    });

    fs::create_dir_all(dest_dir)
        .map_err(|e| format!("Failed to create toolchain directory: {}", e))?;

    let response = ureq::get(&download_url).call().map_err(|e| {
        format!(
            "Failed to download toolchain bundle from {}: {}\n",
            download_url, e
        )
    })?;

    let total_bytes: Option<usize> = response
        .header("Content-Length")
        .and_then(|h| h.parse::<usize>().ok());

    let temp_archive_path = dest_dir.join(format!("parsu_bundle.{}", ARCHIVE_EXTENSION));
    let start_time = Instant::now();

    let mut hasher = Sha256::new();

    {
        let mut reader = response.into_reader();
        let mut out_file = File::create(&temp_archive_path)
            .map_err(|e| format!("Failed to create temporary archive: {}", e))?;

        let mut buffer = [0u8; 64 * 1024];
        let mut downloaded_bytes: usize = 0;

        loop {
            let bytes_read = reader.read(&mut buffer).map_err(|e| e.to_string())?;
            if bytes_read == 0 {
                break;
            }
            out_file
                .write_all(&buffer[..bytes_read])
                .map_err(|e| e.to_string())?;

            hasher.update(&buffer[..bytes_read]);

            downloaded_bytes += bytes_read;

            render_progress(downloaded_bytes, total_bytes);
        }

        out_file.flush().map_err(|e| e.to_string())?;
        eprintln!();
    }

    let computed_hash = format!("{:x}", hasher.finalize());

    if !EXPECTED_SHA256.is_empty() && computed_hash != EXPECTED_SHA256 {
        let _ = fs::remove_file(&temp_archive_path);
        return Err(format!(
            "Toolchain download checksum mismatch! The file is corrupted or tampered.\nExpected: {}\nComputed: {}",
            EXPECTED_SHA256, computed_hash
        ));
    }

    unpack_archive(&temp_archive_path, dest_dir)?;
    let _ = fs::remove_file(&temp_archive_path);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(entries) = fs::read_dir(dest_dir) {
            for entry in entries.flatten() {
                let _ = fs::set_permissions(entry.path(), fs::Permissions::from_mode(0o755));
            }
        }
    }

    let duration = start_time.elapsed().as_secs_f32();
    eprintln!(
        "\x1b[1;92m{:>12}\x1b[0m toolchain bundle to \x1b[2m{}\x1b[0m",
        "Installing",
        dest_dir.display()
    );
    eprintln!(
        "\x1b[1;92m{:>12}\x1b[0m Parsu GPU toolchain v{} in {:.2}s\n",
        "Finished", PARSU_VERSION, duration
    );

    Ok(())
}

fn unpack_archive(archive_path: &Path, dest_dir: &Path) -> Result<(), String> {
    let file = File::open(archive_path).map_err(|e| format!("Failed to open archive: {}", e))?;
    let tar = GzDecoder::new(file);
    let mut archive = Archive::new(tar);
    archive
        .unpack(dest_dir)
        .map_err(|e| format!("Failed to extract toolchain tar.gz: {}", e))?;
    Ok(())
}

fn render_progress(downloaded: usize, total: Option<usize>) {
    let mb_down = downloaded as f64 / (1024.0 * 1024.0);
    const BAR_WIDTH: usize = 28;

    if let Some(tot) = total {
        let mb_tot = tot as f64 / (1024.0 * 1024.0);
        let progress = (downloaded as f64 / tot as f64).clamp(0.0, 1.0);
        let filled = (progress * BAR_WIDTH as f64) as usize;
        let empty = BAR_WIDTH.saturating_sub(filled);

        let bar_filled = "=".repeat(filled.saturating_sub(1));
        let head = if filled > 0 { ">" } else { "" };
        let bar_empty = " ".repeat(empty);

        eprint!(
            "\r\x1b[1;92m{:>12}\x1b[0m parsu v{} [{}{}{}] {:3.0}% ({:.1} MB / {:.1} MB)\x1b[K",
            "Downloading",
            PARSU_VERSION,
            bar_filled,
            head,
            bar_empty,
            progress * 100.0,
            mb_down,
            mb_tot
        );
    } else {
        eprint!(
            "\r\x1b[1;92m{:>12}\x1b[0m parsu v{} ({:.2} MB)\x1b[K",
            "Downloading", PARSU_VERSION, mb_down
        );
    }
    let _ = io::stderr().flush();
}
