use anyhow::{Context, Result};
use ash::vk;
use parsu::baker::AnutuBaker;
use parsu::drivers::DriverSpecializer;
use parsu::profile::HardwareProfile;
use std::any::Any;
use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

use apsu::{
    ApsuAllocator, ApsuArena, GpuDeviceBuffer, GpuImage, GpuSampler, GpuTransferManager,
    slots::{SlotEvent, SlotType},
};

use utu::vulkan::{
    VulkanCommandPool, VulkanDescriptorPool, VulkanDescriptorSetLayout, VulkanDevice,
    VulkanInstance, VulkanQueue, VulkanSemaphore,
};

use crate::context::ring::TimelineCommandRing;
use crate::pipeline_synthesis::PipelineSynthesizer;
use crate::recording::compiler::FrameCompiler;
use crate::recording::queue::TaskQueue;
use crate::recording::recipe::CompiledExecutionRecipe;

pub struct EnkiEngine {
    pub descriptor_pool: VulkanDescriptorPool,
    pub descriptor_layout: VulkanDescriptorSetLayout,
    pub descriptor_set: vk::DescriptorSet,

    pub dummy_buffer: GpuDeviceBuffer,
    pub dummy_image: GpuImage,
    pub dummy_sampler: GpuSampler,
    pub transfer_manager: GpuTransferManager,

    pub param_arena: ApsuArena,

    pub _transfer_command_pool: VulkanCommandPool,

    pub driver_specializer: Arc<dyn DriverSpecializer>,
    pub hardware_profile: HardwareProfile,
    pub pipeline_cache: Mutex<vk::PipelineCache>,
    pub pipeline_cache_uuid: [u8; 16],

    pub synthesizer: PipelineSynthesizer,

    pub recipe_cache: Mutex<HashMap<u64, Arc<CompiledExecutionRecipe>>>,

    pub query_pool: vk::QueryPool,
    pub max_timestamp_queries: u32,
    pub timestamp_period: f32,

    pub timeline_semaphore: VulkanSemaphore,
    pub timeline_counter: AtomicU64,
    pub command_ring: Mutex<TimelineCommandRing>,
    pub resource_cache: Mutex<HashMap<u64, Arc<dyn Any + Send + Sync>>>,

    pub allocator: Arc<ApsuAllocator>,
    pub queue: VulkanQueue,
    pub device: Arc<VulkanDevice>,
    pub instance: Arc<VulkanInstance>,
}

impl EnkiEngine {
    pub fn compile_recipe<'a>(&self, queue: &TaskQueue<'a>) -> Arc<CompiledExecutionRecipe> {
        let structure_hash = queue.calculate_structure_hash();

        let mut cache = self.recipe_cache.lock().unwrap();

        if let Some(recipe) = cache.get(&structure_hash) {
            return recipe.clone();
        }

        let _ = self.try_load_baked_anutu(structure_hash);

        let new_recipe =
            FrameCompiler::compile(queue, self.descriptor_set, &self.synthesizer.cache);

        let recipe_arc = Arc::new(new_recipe);
        cache.insert(structure_hash, recipe_arc.clone());

        recipe_arc
    }

    pub fn try_bake_anutu(&self, structure_hash: u64) -> Result<()> {
        let device = self.raw_device();

        let cache_lock = self.pipeline_cache.lock().unwrap();
        let pipeline_cache_data = unsafe {
            device
                .get_pipeline_cache_data(*cache_lock)
                .context("[Anu Engine] Failed to serialize VkPipelineCache data")?
        };

        let blueprint_bytes = Vec::new();

        let filename = format!("cache_{}.anutu", structure_hash);
        let cache_dir = std::path::Path::new("target/enki_cache");
        std::fs::create_dir_all(cache_dir)?;
        let file_path = cache_dir.join(filename);

        AnutuBaker::bake_anutu(
            &file_path,
            &self.hardware_profile,
            &pipeline_cache_data,
            &blueprint_bytes,
        )?;

        Ok(())
    }

    pub fn try_load_baked_anutu(&self, structure_hash: u64) -> Option<()> {
        let filename = format!("cache_{}.anutu", structure_hash);
        let file_path = std::path::Path::new("target/enki_cache").join(filename);

        if !file_path.exists() {
            return None;
        }

        let (baked_profile, pipeline_cache_data, _blueprint_bytes) =
            AnutuBaker::load_anutu(&file_path).ok()?;

        if baked_profile.vendor_id != self.hardware_profile.vendor_id
            || baked_profile.device_id != self.hardware_profile.device_id
            || baked_profile.quad_operations_in_all_stages
                != self.hardware_profile.quad_operations_in_all_stages
        {
            println!(
                "[Anu Engine] Cache Invalidation: Hardware mismatch detected. Rebuilding .anutu..."
            );
            let _ = std::fs::remove_file(file_path);
            return None;
        }

        let device = self.raw_device();
        unsafe {
            let mut cache_lock = self.pipeline_cache.lock().unwrap();
            device.destroy_pipeline_cache(*cache_lock, None);

            let create_info =
                vk::PipelineCacheCreateInfo::default().initial_data(&pipeline_cache_data);
            *cache_lock = device.create_pipeline_cache(&create_info, None).ok()?;
        }

        Some(())
    }

    pub fn raw_device(&self) -> &ash::Device {
        &self.device.logical_device
    }

    pub fn raw_instance(&self) -> &ash::Instance {
        &self.instance.instance
    }

    pub fn raw_physical_device(&self) -> vk::PhysicalDevice {
        self.allocator.physical_device
    }

    pub fn wait_idle(&self) -> Result<()> {
        unsafe {
            self.device
                .logical_device
                .device_wait_idle()
                .map_err(|e| anyhow::anyhow!("[EnkiEngine] device_wait_idle failed: {}", e))
        }
    }

    pub fn flush_descriptor_updates(&self) -> Result<()> {
        let events = self.allocator.pull_descriptor_events();
        if events.is_empty() {
            return Ok(());
        }

        let mut image_infos = Vec::with_capacity(events.len());

        for event in &events {
            match event {
                SlotEvent::BindImage {
                    image_view,
                    image_layout,
                    ..
                } => {
                    image_infos.push(
                        vk::DescriptorImageInfo::default()
                            .image_view(*image_view)
                            .image_layout(*image_layout),
                    );
                }
                SlotEvent::BindSampler { sampler, .. } => {
                    image_infos.push(vk::DescriptorImageInfo::default().sampler(*sampler));
                }
                SlotEvent::Unbind { slot_type, .. } => match slot_type {
                    SlotType::SampledImage | SlotType::StorageImage => {
                        image_infos.push(
                            vk::DescriptorImageInfo::default()
                                .image_view(self.dummy_image.view)
                                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL),
                        );
                    }
                    SlotType::Sampler => {
                        image_infos.push(
                            vk::DescriptorImageInfo::default().sampler(self.dummy_sampler.sampler),
                        );
                    }
                },
            }
        }

        let mut write_sets = Vec::with_capacity(events.len());
        let mut image_idx = 0;

        for event in &events {
            let mut write = vk::WriteDescriptorSet::default().dst_set(self.descriptor_set);

            match event {
                SlotEvent::BindImage {
                    slot_type,
                    slot_index,
                    ..
                } => {
                    let (binding, descriptor_type) = match slot_type {
                        SlotType::SampledImage => (2, vk::DescriptorType::SAMPLED_IMAGE),
                        SlotType::StorageImage => (3, vk::DescriptorType::STORAGE_IMAGE),
                        _ => unreachable!(),
                    };

                    write = write
                        .dst_binding(binding)
                        .dst_array_element(*slot_index)
                        .descriptor_type(descriptor_type)
                        .image_info(std::slice::from_ref(&image_infos[image_idx]));
                    image_idx += 1;
                }
                SlotEvent::BindSampler { slot_index, .. } => {
                    write = write
                        .dst_binding(4)
                        .dst_array_element(*slot_index)
                        .descriptor_type(vk::DescriptorType::SAMPLER)
                        .image_info(std::slice::from_ref(&image_infos[image_idx]));
                    image_idx += 1;
                }
                SlotEvent::Unbind {
                    slot_type,
                    slot_index,
                } => {
                    let (binding, descriptor_type) = match slot_type {
                        SlotType::SampledImage => (2, vk::DescriptorType::SAMPLED_IMAGE),
                        SlotType::StorageImage => (3, vk::DescriptorType::STORAGE_IMAGE),
                        SlotType::Sampler => (4, vk::DescriptorType::SAMPLER),
                    };

                    write = write
                        .dst_binding(binding)
                        .dst_array_element(*slot_index)
                        .descriptor_type(descriptor_type)
                        .image_info(std::slice::from_ref(&image_infos[image_idx]));
                    image_idx += 1;
                }
            }
            write_sets.push(write);
        }

        unsafe {
            self.device
                .logical_device
                .update_descriptor_sets(&write_sets, &[]);
        }

        Ok(())
    }

    pub fn get_timestamp_query_result(&self, query_index: u32) -> Result<u64> {
        let mut results = [0u64; 1];
        unsafe {
            self.raw_device()
                .get_query_pool_results(
                    self.query_pool,
                    query_index,
                    &mut results,
                    vk::QueryResultFlags::TYPE_64 | vk::QueryResultFlags::WAIT,
                )
                .context("[EnkiEngine] Failed to retrieve timestamp query results from Vulkan")?;
        }
        Ok(results[0])
    }
}

impl Drop for EnkiEngine {
    fn drop(&mut self) {
        let _ = self.wait_idle();

        let artifacts = self.synthesizer.cache.drain_all();
        let cache_lock = self.pipeline_cache.lock().unwrap();
        let device = self.raw_device();

        unsafe {
            device.destroy_pipeline_cache(*cache_lock, None);
            device.destroy_query_pool(self.query_pool, None);

            for artifact in artifacts {
                device.destroy_pipeline(artifact.pipeline, None);
                device.destroy_pipeline_layout(artifact.layout, None);
            }
        }
    }
}
