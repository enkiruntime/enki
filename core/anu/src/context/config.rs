use std::ffi::CString;

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub app_name: String,
    pub caller_location: Option<(&'static str, u32, u32)>,

    pub max_sampled_images: u32,
    pub max_storage_images: u32,
    pub max_samplers: u32,

    pub max_param_arena_size: Option<u64>,
    pub max_timestamp_queries: u32,

    pub required_instance_extensions: Vec<CString>,
    pub required_device_extensions: Vec<CString>,

    pub headless: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            app_name: "Enki Engine".to_string(),
            caller_location: None,
            max_sampled_images: 1024,
            max_storage_images: 512,
            max_samplers: 64,

            max_param_arena_size: Some(128 * 128 * 128),
            max_timestamp_queries: 128,

            required_instance_extensions: Vec::new(),
            required_device_extensions: Vec::new(),
            headless: false,
        }
    }
}
