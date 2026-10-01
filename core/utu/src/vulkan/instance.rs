use anyhow::{Context, Result};
use ash::vk;
use std::ffi::{CStr, CString};

pub struct VulkanInstance {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub debug_loader: Option<ash::ext::debug_utils::Instance>,
    pub debug_messenger: Option<vk::DebugUtilsMessengerEXT>,
}

pub struct VulkanInstanceBuilder<'a> {
    app_name: String,
    required_extensions: Vec<&'a CStr>,
    enable_validation: bool,
}

impl<'a> VulkanInstanceBuilder<'a> {
    pub fn new(app_name: &str) -> Self {
        Self {
            app_name: app_name.to_string(),
            required_extensions: Vec::new(),
            enable_validation: cfg!(debug_assertions),
        }
    }

    pub fn with_extensions(mut self, extensions: &[&'a CStr]) -> Self {
        self.required_extensions.extend_from_slice(extensions);
        self
    }

    pub fn enable_validation(mut self, enable: bool) -> Self {
        self.enable_validation = enable;
        self
    }

    pub fn build(self) -> Result<VulkanInstance> {
        let entry = unsafe {
            ash::Entry::load().context("[VulkanInstance] Failed to load Vulkan entry library")?
        };

        let available_layers = unsafe {
            entry
                .enumerate_instance_layer_properties()
                .unwrap_or_default()
        };

        let validation_layer_name =
            unsafe { CStr::from_bytes_with_nul_unchecked(b"VK_LAYER_KHRONOS_validation\0") };

        let has_validation_layer = available_layers.iter().any(|layer| {
            let name = unsafe { CStr::from_ptr(layer.layer_name.as_ptr()) };
            name == validation_layer_name
        });

        let available_extensions = unsafe {
            entry
                .enumerate_instance_extension_properties(None)
                .unwrap_or_default()
        };

        let has_debug_utils = available_extensions.iter().any(|ext| {
            let name = unsafe { CStr::from_ptr(ext.extension_name.as_ptr()) };
            name == ash::ext::debug_utils::NAME
        });

        let should_enable_validation =
            self.enable_validation && has_validation_layer && has_debug_utils;

        let mut extension_names: Vec<*const std::os::raw::c_char> = self
            .required_extensions
            .iter()
            .map(|ext| ext.as_ptr())
            .collect();

        if should_enable_validation {
            extension_names.push(ash::ext::debug_utils::NAME.as_ptr());
        }

        let app_name_cstr = CString::new(self.app_name.as_str())
            .context("[VulkanInstance] Failed to allocate CString for app name")?;

        let app_info = vk::ApplicationInfo::default()
            .application_name(app_name_cstr.as_c_str())
            .application_version(vk::make_api_version(0, 1, 0, 0))
            .engine_name(app_name_cstr.as_c_str())
            .engine_version(vk::make_api_version(0, 1, 0, 0))
            .api_version(vk::API_VERSION_1_3);

        let mut create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&extension_names);

        let layers = [validation_layer_name.as_ptr()];

        if should_enable_validation {
            create_info = create_info.enabled_layer_names(&layers);
        }

        let instance = unsafe {
            entry
                .create_instance(&create_info, None)
                .context("[VulkanInstance] Failed to create Vulkan raw Instance")?
        };

        let mut debug_loader = None;
        let mut debug_messenger = None;

        if should_enable_validation {
            let loader = ash::ext::debug_utils::Instance::new(&entry, &instance);

            let messenger_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
                .message_severity(
                    vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                        | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING,
                )
                .message_type(
                    vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                        | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                        | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
                )
                .pfn_user_callback(Some(vulkan_debug_callback));

            let messenger = unsafe {
                loader
                    .create_debug_utils_messenger(&messenger_info, None)
                    .context("[VulkanInstance] Failed to create Debug Utils Messenger")?
            };

            debug_loader = Some(loader);
            debug_messenger = Some(messenger);
        }

        Ok(VulkanInstance {
            entry,
            instance,
            debug_loader,
            debug_messenger,
        })
    }
}

impl Drop for VulkanInstance {
    fn drop(&mut self) {
        unsafe {
            if let Some(ref loader) = self.debug_loader {
                if let Some(messenger) = self.debug_messenger {
                    loader.destroy_debug_utils_messenger(messenger, None);
                }
            }
            self.instance.destroy_instance(None);
        }
    }
}

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let message = unsafe {
        let callback_data = *p_callback_data;
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    if message_severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::ERROR) {
        eprintln!("\x1b[1;31m[Vulkan Validation Error]\x1b[0m {}", message);
    }

    vk::FALSE
}
