pub mod cache;
pub mod compiler;
pub mod coordinator;
pub mod types;

use anyhow::Result;
pub use cache::{RAMTaskCache, calculate_compute_key, calculate_source_hash};
pub use coordinator::JitCoordinator;
pub use types::{ComputeSynthesisInput, TaskCompilationArtifact};

use crate::context::EnkiEngine;

pub struct PipelineSynthesizer {
    pub cache: RAMTaskCache,
}

impl PipelineSynthesizer {
    pub fn new() -> Self {
        Self {
            cache: RAMTaskCache::new(),
        }
    }

    pub fn synthesize_compute(
        &self,
        engine: &EnkiEngine,
        input: &ComputeSynthesisInput,
    ) -> Result<TaskCompilationArtifact> {
        let source_hash = calculate_source_hash(&input.nam_name);
        let cache_key = calculate_compute_key(
            source_hash,
            &input.nam_name,
            input.local_size,
            &input.arg_descriptors,
        );

        if let Some(cached_artifact) = self.cache.get(cache_key) {
            return Ok(cached_artifact);
        }

        JitCoordinator::orchestrate(engine, input)?;

        if let Some(cached_artifact) = self.cache.get(cache_key) {
            return Ok(cached_artifact);
        }

        Err(anyhow::anyhow!(
            "[PipelineSynthesizer] JIT Pipeline cache miss for nam '{}'. Please rebuild your project using 'cargo build'.",
            input.nam_name
        ))
    }
}

impl Default for PipelineSynthesizer {
    fn default() -> Self {
        Self::new()
    }
}
