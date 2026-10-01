use crate::nam_args_api::ArgDescriptor;
use ash::vk;

#[derive(Debug, Clone)]
pub struct ComputeSynthesisInput {
    pub nam_name: String,
    pub local_size: (u32, u32, u32),
    pub host_manifest_dir: Option<String>,
    pub caller_file_path: Option<String>,
    pub arg_descriptors: Vec<ArgDescriptor>,
}

#[derive(Debug, Clone)]
pub struct TaskCompilationArtifact {
    pub pipeline: vk::Pipeline,
    pub layout: vk::PipelineLayout,
    pub local_size: (u32, u32, u32),
    pub stack_size_per_thread: u32,
}
