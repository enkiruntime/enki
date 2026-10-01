use anyhow::Result;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::RwLock;

use crate::nam_args_api::ArgDescriptor;
use crate::pipeline_synthesis::types::TaskCompilationArtifact;
use parsu::baker::AnutuBaker;
use parsu::profile::HardwareProfile;

pub fn calculate_source_hash(source: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    hasher.finish()
}

pub fn calculate_compute_key(
    source_hash: u64,
    entry_point: &str,
    local_size: (u32, u32, u32),
    arg_descriptors: &[ArgDescriptor],
) -> u64 {
    let mut hasher = DefaultHasher::new();
    source_hash.hash(&mut hasher);
    entry_point.hash(&mut hasher);
    local_size.hash(&mut hasher);

    for desc in arg_descriptors {
        desc.hash(&mut hasher);
    }

    hasher.finish()
}

#[derive(Default)]
pub struct RAMTaskCache {
    cache: RwLock<HashMap<u64, TaskCompilationArtifact>>,
}

impl RAMTaskCache {
    pub fn new() -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
        }
    }

    pub fn insert(&self, key: u64, artifact: TaskCompilationArtifact) {
        let mut lock = self.cache.write().unwrap();
        lock.insert(key, artifact);
    }

    pub fn get(&self, key: u64) -> Option<TaskCompilationArtifact> {
        let lock = self.cache.read().unwrap();
        lock.get(&key).cloned()
    }

    pub fn get_by_pipeline(&self, pipeline: ash::vk::Pipeline) -> Option<TaskCompilationArtifact> {
        let lock = self.cache.read().unwrap();
        lock.values().find(|a| a.pipeline == pipeline).cloned()
    }

    pub fn drain_all(&self) -> Vec<TaskCompilationArtifact> {
        let mut lock = self.cache.write().unwrap();
        lock.drain().map(|(_, v)| v).collect()
    }
}

pub struct DiskCacheManager {
    cache_dir: PathBuf,
}

impl DiskCacheManager {
    pub fn new() -> Self {
        let user_project_dir = std::env::var("CARGO_MANIFEST_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());

        Self {
            cache_dir: user_project_dir.join("target").join("enki_cache"),
        }
    }

    pub fn get_cache_path(&self, hash_key: u64) -> PathBuf {
        self.cache_dir.join(format!("cache_{}.anutu", hash_key))
    }

    pub fn save_cache(
        &self,
        hash_key: u64,
        profile: &HardwareProfile,
        pipeline_cache_data: &[u8],
        spirv_bytes: &[u8],
        stack_size_per_thread: u32,
    ) -> Result<()> {
        fs::create_dir_all(&self.cache_dir)?;
        let file_path = self.get_cache_path(hash_key);

        let mut payload = Vec::with_capacity(4 + spirv_bytes.len());
        payload.extend_from_slice(&stack_size_per_thread.to_ne_bytes());
        payload.extend_from_slice(spirv_bytes);

        AnutuBaker::bake_anutu(&file_path, profile, pipeline_cache_data, &payload)?;
        Ok(())
    }

    pub fn load_cache(
        &self,
        hash_key: u64,
        current_profile: &HardwareProfile,
    ) -> Option<(Vec<u8>, Vec<u8>, u32)> {
        let file_path = self.get_cache_path(hash_key);
        if !file_path.exists() {
            return None;
        }

        let (baked_profile, pipeline_cache_data, payload) =
            AnutuBaker::load_anutu(&file_path).ok()?;

        if baked_profile.vendor_id != current_profile.vendor_id
            || baked_profile.device_id != current_profile.device_id
        {
            let _ = fs::remove_file(file_path);
            return None;
        }

        if payload.len() < 4 {
            return None;
        }

        let stack_size_per_thread = u32::from_ne_bytes(payload[0..4].try_into().ok()?);
        let spirv_bytes = payload[4..].to_vec();

        Some((pipeline_cache_data, spirv_bytes, stack_size_per_thread))
    }
}
impl Default for DiskCacheManager {
    fn default() -> Self {
        Self::new()
    }
}
