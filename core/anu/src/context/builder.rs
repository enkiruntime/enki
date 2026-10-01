use anyhow::{Context, Result, anyhow};
use ash::vk;
use std::collections::HashMap;
use std::ffi::CStr;
use std::sync::{Arc, Mutex};

use apsu::{
    ApsuAllocator, ApsuArena, BufferUsage, GpuDeviceBuffer, GpuImage, GpuSampler,
    GpuTransferManager,
};

use utu::vulkan::{
    DescriptorPoolBuilder, DescriptorSetLayoutBuilder, PhysicalDeviceInfo, VulkanCommandPool,
    VulkanDeviceBuilder, VulkanInstanceBuilder, VulkanSemaphore,
};

use parsu::drivers::{self};
use parsu::profile::ProfileQuerier;

use super::config::EngineConfig;
use super::engine::EnkiEngine;
use super::ring::TimelineCommandRing;
use crate::diagnostics::{emit_diagnostic, hw};
use crate::pipeline_synthesis::PipelineSynthesizer;

pub struct EnkiEngineBuilder {
    config: EngineConfig,
}

impl EnkiEngineBuilder {
    pub fn new(config: EngineConfig) -> Self {
        Self { config }
    }

    pub fn build(self) -> Result<EnkiEngine> {
        let required_instance_extensions: Vec<&CStr> = self
            .config
            .required_instance_extensions
            .iter()
            .map(|c| c.as_c_str())
            .collect();

        let instance = match VulkanInstanceBuilder::new(self.config.app_name.as_str())
            .with_extensions(&required_instance_extensions)
            .build()
        {
            Ok(inst) => Arc::new(inst),
            Err(e) => {
                let diag = hw::driver_not_found(e.to_string());
                return Err(anyhow!("{}", emit_diagnostic(&diag)));
            }
        };

        let physical_devices = unsafe {
            instance
                .instance
                .enumerate_physical_devices()
                .map_err(|e| {
                    let diag =
                        hw::driver_not_found(format!("Failed to enumerate physical devices: {e}"));
                    anyhow!("{}", emit_diagnostic(&diag))
                })?
        };

        if physical_devices.is_empty() {
            let diag = hw::zero_gpus_found();
            return Err(anyhow!("{}", emit_diagnostic(&diag)));
        }

        let devices_info: Vec<PhysicalDeviceInfo> = physical_devices
            .into_iter()
            .map(|pd| PhysicalDeviceInfo::query(&instance.instance, pd))
            .collect();

        let selected_gpu_info = devices_info
            .iter()
            .find(|info| info.properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU)
            .or_else(|| devices_info.first())
            .ok_or_else(|| {
                let diag = hw::zero_gpus_found();
                anyhow!("{}", emit_diagnostic(&diag))
            })?;

        let selected_name = unsafe {
            CStr::from_ptr(selected_gpu_info.properties.device_name.as_ptr())
                .to_string_lossy()
                .into_owned()
        };

        if selected_gpu_info.properties.device_type == vk::PhysicalDeviceType::INTEGRATED_GPU {
            let dgpu_name = devices_info
                .iter()
                .find(|d| d.properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU)
                .map(|d| unsafe {
                    CStr::from_ptr(d.properties.device_name.as_ptr())
                        .to_string_lossy()
                        .into_owned()
                });

            if dgpu_name.is_some() {
                let warn_diag = hw::integrated_gpu_alert(selected_name.clone(), dgpu_name);
                eprint!("{}", emit_diagnostic(&warn_diag));
            }
        }

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
                    .get_physical_device_features2(selected_gpu_info.handle, &mut query_features2);
            }
            supported_base_features = query_features2.features;
        }

        let mut missing_features = Vec::new();
        if supported_features12.buffer_device_address != vk::TRUE {
            missing_features.push("64-bit Buffer Device Addresses (BDA)");
        }
        if supported_features12.timeline_semaphore != vk::TRUE {
            missing_features.push("Timeline Semaphores");
        }
        if supported_features13.synchronization2 != vk::TRUE {
            missing_features.push("Synchronization2");
        }
        if supported_base_features.shader_int64 != vk::TRUE {
            missing_features.push("64-bit Integer Shader Operations (shaderInt64)");
        }

        if !missing_features.is_empty() {
            let diag = hw::missing_features(selected_name, missing_features);
            return Err(anyhow!("{}", emit_diagnostic(&diag)));
        }

        let vendor_id = selected_gpu_info.properties.vendor_id;
        let driver_specializer = drivers::select_driver(vendor_id);

        let hardware_profile = ProfileQuerier::query_profile(
            &instance.entry,
            &instance.instance,
            selected_gpu_info.handle,
        )
        .context("[EnkiBuilder] Failed to query hardware profile from Parsu HAL")?;

        let queue_family_idx = selected_gpu_info
            .queue_families
            .iter()
            .position(|q| {
                q.queue_flags
                    .contains(vk::QueueFlags::COMPUTE | vk::QueueFlags::TRANSFER)
            })
            .or_else(|| {
                selected_gpu_info
                    .queue_families
                    .iter()
                    .position(|q| q.queue_flags.contains(vk::QueueFlags::COMPUTE))
            })
            .unwrap_or(0) as u32;

        let required_device_extensions: Vec<&CStr> = self
            .config
            .required_device_extensions
            .iter()
            .map(|c| c.as_c_str())
            .collect();

        let (device, mut queues) = VulkanDeviceBuilder::new(selected_gpu_info.handle)
            .request_queue(queue_family_idx, vec![1.0])
            .enable_buffer_device_address(true)
            .enable_descriptor_indexing(true)
            .with_extensions(&required_device_extensions)
            .build(&instance)
            .context("[EnkiBuilder] Failed to build Vulkan Logical Device")?;

        let device = Arc::new(device);
        let queue = queues.remove(0);

        let mut allocator_raw =
            ApsuAllocator::new(&instance.instance, selected_gpu_info.handle, &device)
                .context("[EnkiBuilder] Failed to initialize Apsu Allocator")?;
        allocator_raw._device_keeper = Some(device.clone());
        let allocator = Arc::new(allocator_raw);

        let descriptor_layout = DescriptorSetLayoutBuilder::new()
            .add_binding(
                2,
                vk::DescriptorType::SAMPLED_IMAGE,
                self.config.max_sampled_images,
                vk::ShaderStageFlags::ALL,
                vk::DescriptorBindingFlags::PARTIALLY_BOUND
                    | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND,
            )
            .add_binding(
                3,
                vk::DescriptorType::STORAGE_IMAGE,
                self.config.max_storage_images,
                vk::ShaderStageFlags::ALL,
                vk::DescriptorBindingFlags::PARTIALLY_BOUND
                    | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND,
            )
            .add_binding(
                4,
                vk::DescriptorType::SAMPLER,
                self.config.max_samplers,
                vk::ShaderStageFlags::ALL,
                vk::DescriptorBindingFlags::PARTIALLY_BOUND
                    | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND,
            )
            .build(&device.logical_device)?;

        let descriptor_pool = DescriptorPoolBuilder::new()
            .add_pool_size(
                vk::DescriptorType::SAMPLED_IMAGE,
                self.config.max_sampled_images,
            )
            .add_pool_size(
                vk::DescriptorType::STORAGE_IMAGE,
                self.config.max_storage_images,
            )
            .add_pool_size(vk::DescriptorType::SAMPLER, self.config.max_samplers)
            .with_max_sets(1)
            .with_flags(vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND)
            .build(&device.logical_device)?;

        let descriptor_set = descriptor_pool.allocate_set(descriptor_layout.handle)?;

        let dummy_buffer = GpuDeviceBuffer::new(
            allocator.clone(),
            16,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::UNIFORM_BUFFER,
        )?;
        let dummy_image = GpuImage::new(
            allocator.clone(),
            1,
            1,
            vk::Format::R8G8B8A8_UNORM,
            vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::STORAGE,
        )?;
        let dummy_sampler = GpuSampler::new(allocator.clone(), &vk::SamplerCreateInfo::default())?;

        let command_pool = VulkanCommandPool::new(
            &device.logical_device,
            queue_family_idx,
            vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
        )?;
        let transfer_manager = GpuTransferManager::new(
            allocator.clone(),
            queue.handle,
            command_pool.handle,
            1024 * 1024 * 128,
        )?;

        let max_device_alloc = selected_gpu_info.properties.limits.max_storage_buffer_range as u64;
        let mut target_arena_size = self.config.max_param_arena_size.unwrap_or(128 * 128 * 128);

        if max_device_alloc > 0 && target_arena_size > max_device_alloc {
            let mut warn_diag = hw::param_arena_clamped(target_arena_size, max_device_alloc);

            if let Some((file, line, col)) = self.config.caller_location {
                warn_diag.add_span(crate::diagnostics::source::Span::primary(
                    file,
                    line as usize,
                    col as usize,
                    1,
                ));
            }

            eprint!("{}", emit_diagnostic(&warn_diag));
            target_arena_size = max_device_alloc;
        }

        let param_arena = ApsuArena::new_device(
            allocator.clone(),
            target_arena_size,
            BufferUsage::SHADER_DEVICE_ADDRESS
                | BufferUsage::UNIFORM_BUFFER
                | BufferUsage::STORAGE_BUFFER,
        )?;

        let ring_capacity = 4usize;
        let max_timestamp_queries = self.config.max_timestamp_queries.max(2);
        let total_query_count = max_timestamp_queries * (ring_capacity as u32);

        let query_pool_info = vk::QueryPoolCreateInfo::default()
            .query_type(vk::QueryType::TIMESTAMP)
            .query_count(total_query_count);

        let query_pool = unsafe {
            device
                .logical_device
                .create_query_pool(&query_pool_info, None)
                .context("[EnkiBuilder] Failed to create Vulkan Query Pool")?
        };

        let properties = unsafe {
            instance
                .instance
                .get_physical_device_properties(selected_gpu_info.handle)
        };
        let timestamp_period = properties.limits.timestamp_period;

        let pipeline_cache_create_info = vk::PipelineCacheCreateInfo::default();
        let pipeline_cache = unsafe {
            device
                .logical_device
                .create_pipeline_cache(&pipeline_cache_create_info, None)
                .context("[EnkiBuilder] Failed to create Vulkan Pipeline Cache")?
        };

        let physical_device_properties = unsafe {
            instance
                .instance
                .get_physical_device_properties(selected_gpu_info.handle)
        };
        let pipeline_cache_uuid = physical_device_properties.pipeline_cache_uuid;

        let timeline_semaphore = VulkanSemaphore::new_timeline(&device.logical_device, 0)?;
        let command_ring = TimelineCommandRing::new(&device.logical_device, queue_family_idx, 4)?;

        let synthesizer = PipelineSynthesizer::new();
        let recipe_cache = Mutex::new(HashMap::new());
        let resource_cache = Mutex::new(HashMap::new());

        let engine = EnkiEngine {
            instance,
            device,
            queue,
            allocator,
            descriptor_pool,
            descriptor_layout,
            descriptor_set,
            dummy_buffer,
            dummy_image,
            dummy_sampler,
            transfer_manager,
            param_arena,
            _transfer_command_pool: command_pool,
            driver_specializer,
            hardware_profile,
            pipeline_cache: Mutex::new(pipeline_cache),
            pipeline_cache_uuid,
            synthesizer,
            recipe_cache,
            resource_cache,
            query_pool,
            max_timestamp_queries,
            timestamp_period,
            timeline_semaphore,
            timeline_counter: std::sync::atomic::AtomicU64::new(0),
            command_ring: Mutex::new(command_ring),
        };

        Ok(engine)
    }
}
