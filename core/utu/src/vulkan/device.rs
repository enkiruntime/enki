use crate::vulkan::VulkanInstance;
use anyhow::{Context, Result};
use ash::vk;
use std::ffi::CStr;
use std::sync::Arc;

pub struct PhysicalDeviceInfo {
    pub handle: vk::PhysicalDevice,
    pub properties: vk::PhysicalDeviceProperties,
    pub features: vk::PhysicalDeviceFeatures,
    pub queue_families: Vec<vk::QueueFamilyProperties>,
}

impl PhysicalDeviceInfo {
    pub fn query(instance: &ash::Instance, handle: vk::PhysicalDevice) -> Self {
        let properties = unsafe { instance.get_physical_device_properties(handle) };
        let features = unsafe { instance.get_physical_device_features(handle) };
        let queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(handle) };

        Self {
            handle,
            properties,
            features,
            queue_families,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VulkanQueue {
    pub handle: vk::Queue,
    pub family_index: u32,
    pub queue_index: u32,
}

#[derive(Clone)]
pub struct VulkanDevice {
    pub logical_device: ash::Device,
    pub instance: Arc<VulkanInstance>,
}

impl Drop for VulkanDevice {
    fn drop(&mut self) {
        unsafe {
            let _ = self.logical_device.device_wait_idle();
            self.logical_device.destroy_device(None);
        }
    }
}

pub struct QueueRequest {
    pub family_index: u32,
    pub priorities: Vec<f32>,
}

pub struct VulkanDeviceBuilder<'a> {
    physical_device: vk::PhysicalDevice,
    required_extensions: Vec<&'a CStr>,
    queue_requests: Vec<QueueRequest>,
    enable_buffer_device_address: bool,
    enable_descriptor_indexing: bool,
}

impl<'a> VulkanDeviceBuilder<'a> {
    pub fn new(physical_device: vk::PhysicalDevice) -> Self {
        Self {
            physical_device,
            required_extensions: Vec::new(),
            queue_requests: Vec::new(),
            enable_buffer_device_address: false,
            enable_descriptor_indexing: false,
        }
    }

    pub fn with_extensions(mut self, extensions: &[&'a CStr]) -> Self {
        self.required_extensions.extend_from_slice(extensions);
        self
    }

    pub fn request_queue(mut self, family_index: u32, priorities: Vec<f32>) -> Self {
        self.queue_requests.push(QueueRequest {
            family_index,
            priorities,
        });
        self
    }

    pub fn enable_buffer_device_address(mut self, enable: bool) -> Self {
        self.enable_buffer_device_address = enable;
        self
    }

    pub fn enable_descriptor_indexing(mut self, enable: bool) -> Self {
        self.enable_descriptor_indexing = enable;
        self
    }

    pub fn build(self, instance: &Arc<VulkanInstance>) -> Result<(VulkanDevice, Vec<VulkanQueue>)> {
        let mut supported_features11 = vk::PhysicalDeviceVulkan11Features::default();
        let mut supported_features12 = vk::PhysicalDeviceVulkan12Features::default();
        let mut supported_features13 = vk::PhysicalDeviceVulkan13Features::default();
        let supported_base_features;

        {
            let mut query_features2 = vk::PhysicalDeviceFeatures2::default()
                .push_next(&mut supported_features11)
                .push_next(&mut supported_features12)
                .push_next(&mut supported_features13);

            unsafe {
                instance
                    .instance
                    .get_physical_device_features2(self.physical_device, &mut query_features2);
            }

            supported_base_features = query_features2.features;

            if self.enable_buffer_device_address
                && supported_features12.buffer_device_address != vk::TRUE
            {
                return Err(anyhow::anyhow!(
                    "[VulkanDevice] Hardware fault: Physical device does not support Buffer Device Address (BDA)"
                ));
            }
            if supported_features12.timeline_semaphore != vk::TRUE {
                return Err(anyhow::anyhow!(
                    "[VulkanDevice] Hardware fault: Physical device does not support Timeline Semaphores"
                ));
            }
            if supported_features13.synchronization2 != vk::TRUE {
                return Err(anyhow::anyhow!(
                    "[VulkanDevice] Hardware fault: Physical device does not support Synchronization2"
                ));
            }
            if supported_base_features.shader_int64 != vk::TRUE {
                return Err(anyhow::anyhow!(
                    "[VulkanDevice] Hardware fault: Physical device does not support 64-bit integer shader operations (shaderInt64)"
                ));
            }

            let queue_create_infos: Vec<vk::DeviceQueueCreateInfo> = self
                .queue_requests
                .iter()
                .map(|req| {
                    vk::DeviceQueueCreateInfo::default()
                        .queue_family_index(req.family_index)
                        .queue_priorities(&req.priorities)
                })
                .collect();

            let extension_names: Vec<*const std::os::raw::c_char> = self
                .required_extensions
                .iter()
                .map(|ext| ext.as_ptr())
                .collect();

            let mut features11 = vk::PhysicalDeviceVulkan11Features::default()
                .variable_pointers(supported_features11.variable_pointers == vk::TRUE)
                .variable_pointers_storage_buffer(
                    supported_features11.variable_pointers_storage_buffer == vk::TRUE,
                );

            let mut features12 = vk::PhysicalDeviceVulkan12Features::default()
                .buffer_device_address(self.enable_buffer_device_address)
                .descriptor_indexing(self.enable_descriptor_indexing)
                .runtime_descriptor_array(self.enable_descriptor_indexing)
                .descriptor_binding_partially_bound(self.enable_descriptor_indexing)
                .descriptor_binding_storage_buffer_update_after_bind(
                    self.enable_descriptor_indexing,
                )
                .descriptor_binding_sampled_image_update_after_bind(self.enable_descriptor_indexing)
                .descriptor_binding_storage_image_update_after_bind(self.enable_descriptor_indexing)
                .descriptor_binding_uniform_buffer_update_after_bind(
                    self.enable_descriptor_indexing,
                )
                .timeline_semaphore(true)
                .scalar_block_layout(supported_features12.scalar_block_layout == vk::TRUE)
                .shader_int8(supported_features12.shader_int8 == vk::TRUE)
                .shader_float16(supported_features12.shader_float16 == vk::TRUE);

            let mut features13 =
                vk::PhysicalDeviceVulkan13Features::default().synchronization2(true);

            let physical_device_features = vk::PhysicalDeviceFeatures::default()
                .shader_int64(true)
                .shader_int16(supported_base_features.shader_int16 == vk::TRUE)
                .shader_float64(supported_base_features.shader_float64 == vk::TRUE);

            let mut features2 = vk::PhysicalDeviceFeatures2::default()
                .push_next(&mut features11)
                .push_next(&mut features12)
                .push_next(&mut features13);

            features2.features = physical_device_features;

            let device_create_info = vk::DeviceCreateInfo::default()
                .queue_create_infos(&queue_create_infos)
                .enabled_extension_names(&extension_names)
                .push_next(&mut features2);

            let logical_device = unsafe {
                instance
                    .instance
                    .create_device(self.physical_device, &device_create_info, None)
                    .context("[VulkanDevice] Failed to create Logical Device")?
            };

            let mut retrieved_queues = Vec::new();
            for req in &self.queue_requests {
                for (queue_idx, _) in req.priorities.iter().enumerate() {
                    let queue_handle = unsafe {
                        logical_device.get_device_queue(req.family_index, queue_idx as u32)
                    };
                    retrieved_queues.push(VulkanQueue {
                        handle: queue_handle,
                        family_index: req.family_index,
                        queue_index: queue_idx as u32,
                    });
                }
            }

            Ok((
                VulkanDevice {
                    logical_device,
                    instance: instance.clone(),
                },
                retrieved_queues,
            ))
        }
    }
}

impl std::ops::Deref for VulkanDevice {
    type Target = ash::Device;
    fn deref(&self) -> &Self::Target {
        &self.logical_device
    }
}
