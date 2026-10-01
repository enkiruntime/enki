pub mod chunk;

pub use chunk::{AnutuChunk, ChunkSerializer};

use crate::profile::HardwareProfile;
use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};

pub const ANUTU_MAGIC_SIGNATURE: &[u8; 6] = b"ANUTU\0";

pub const MAX_CACHED_ANUTU_FILES: usize = 32;

pub struct AnutuBaker;

impl AnutuBaker {
    pub fn bake_anutu(
        file_path: &Path,
        hardware_profile: &HardwareProfile,
        pipeline_cache_bytes: &[u8],
        code_payload_bytes: &[u8],
    ) -> Result<()> {
        let meta_bytes = serde_json::to_vec(hardware_profile)
            .context("[AnutuBaker] Failed to serialize hardware metadata")?;

        let meta_chunk = AnutuChunk::new(*b"META", meta_bytes);
        let pipe_chunk = AnutuChunk::new(*b"PIPE", pipeline_cache_bytes.to_vec());
        let code_chunk = AnutuChunk::new(*b"CODE", code_payload_bytes.to_vec());

        let serialized_chunks =
            ChunkSerializer::serialize_chunks(&[meta_chunk, pipe_chunk, code_chunk]);

        let mut final_file_bytes =
            Vec::with_capacity(serialized_chunks.len() + ANUTU_MAGIC_SIGNATURE.len());
        final_file_bytes.extend_from_slice(ANUTU_MAGIC_SIGNATURE);
        final_file_bytes.extend_from_slice(&serialized_chunks);

        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "[AnutuBaker] Failed to create cache directory at {:?}",
                    parent
                )
            })?;
        }

        fs::write(file_path, final_file_bytes).with_context(|| {
            format!(
                "[AnutuBaker] Failed to write .anutu file to disk at {:?}",
                file_path
            )
        })?;

        if let Some(cache_dir) = file_path.parent() {
            Self::evict_stale_caches(cache_dir, MAX_CACHED_ANUTU_FILES);
        }

        Ok(())
    }

    pub fn load_anutu(file_path: &Path) -> Result<(HardwareProfile, Vec<u8>, Vec<u8>)> {
        let file_bytes = fs::read(file_path).with_context(|| {
            format!(
                "[AnutuBaker] Failed to read .anutu file from {:?}",
                file_path
            )
        })?;

        if file_bytes.len() < ANUTU_MAGIC_SIGNATURE.len() {
            return Err(anyhow!("[AnutuBaker] Invalid .anutu file: file too short"));
        }

        if &file_bytes[..ANUTU_MAGIC_SIGNATURE.len()] != ANUTU_MAGIC_SIGNATURE {
            return Err(anyhow!(
                "[AnutuBaker] Invalid magic signature: expected 'ANUTU\\0', found '{:?}'",
                String::from_utf8_lossy(&file_bytes[..ANUTU_MAGIC_SIGNATURE.len()])
            ));
        }

        let chunk_stream = &file_bytes[ANUTU_MAGIC_SIGNATURE.len()..];
        let chunks = ChunkSerializer::deserialize_chunks(chunk_stream)
            .context("[AnutuBaker] Failed to parse chunks from byte stream")?;

        let mut hardware_profile: Option<HardwareProfile> = None;
        let mut pipeline_cache_bytes: Option<Vec<u8>> = None;
        let mut code_payload_bytes: Option<Vec<u8>> = None;

        for chunk in &chunks {
            match &chunk.chunk_type {
                b"META" => {
                    let profile: HardwareProfile = serde_json::from_slice(&chunk.payload).context(
                        "[AnutuBaker] Failed to deserialize HardwareProfile from META chunk",
                    )?;
                    hardware_profile = Some(profile);
                }
                b"PIPE" => {
                    pipeline_cache_bytes = Some(chunk.payload.clone());
                }
                b"CODE" | b"BLUE" => {
                    code_payload_bytes = Some(chunk.payload.clone());
                }
                _ => {}
            }
        }

        let hardware_profile = hardware_profile
            .ok_or_else(|| anyhow!("[AnutuBaker] Corrupted .anutu file: META chunk missing"))?;

        let pipeline_cache_bytes = pipeline_cache_bytes
            .ok_or_else(|| anyhow!("[AnutuBaker] Corrupted .anutu file: PIPE chunk missing"))?;

        let code_payload_bytes = code_payload_bytes
            .ok_or_else(|| anyhow!("[AnutuBaker] Corrupted .anutu file: CODE chunk missing"))?;

        Ok((hardware_profile, pipeline_cache_bytes, code_payload_bytes))
    }

    fn evict_stale_caches(cache_dir: &Path, max_allowed: usize) {
        let entries = match fs::read_dir(cache_dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        let mut anutu_files: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "anutu") {
                if let Ok(meta) = entry.metadata() {
                    if let Ok(mtime) = meta.modified() {
                        anutu_files.push((path, mtime));
                    }
                }
            }
        }

        if anutu_files.len() > max_allowed {
            anutu_files.sort_by_key(|(_, mtime)| *mtime);

            let excess_count = anutu_files.len() - max_allowed;
            for (path_to_delete, _) in anutu_files.into_iter().take(excess_count) {
                let _ = fs::remove_file(path_to_delete);
            }
        }
    }
}
