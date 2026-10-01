use anyhow::{Context, Result};
use ash::vk;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};
use vk_mem::Allocator;

use crate::resources::GpuMemoryStats;
use crate::slots::pool::{SlotEvent, SlotPool, SlotType};

pub enum DeletableResource {
    Buffer(vk::Buffer, vk_mem::Allocation),
    ImageView(vk::ImageView),
    Image(vk::Image, vk_mem::Allocation),
    Sampler(vk::Sampler),
    Slot { slot_type: SlotType, index: u32 },
}

pub struct DeferredDeletion {
    pub resource: DeletableResource,
    pub delete_after_timeline_value: u64,
}

pub struct ApsuAllocator {
    pub allocator: Arc<Allocator>,
    pub device: ash::Device,
    pub physical_device: vk::PhysicalDevice,
    pub is_unified_memory: bool,

    pub slot_pool: SlotPool,
    pub pending_events: Mutex<Vec<SlotEvent>>,
    pub _device_keeper: Option<Arc<dyn std::any::Any + Send + Sync>>,

    pub current_timeline_value: Arc<AtomicU64>,
    pub deferred_deletions: Mutex<Vec<DeferredDeletion>>,
}

impl ApsuAllocator {
    pub fn new(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        device: &ash::Device,
    ) -> Result<Self> {
        let device_properties = unsafe { instance.get_physical_device_properties(physical_device) };

        let memory_properties =
            unsafe { instance.get_physical_device_memory_properties(physical_device) };

        let mut is_unified_memory =
            device_properties.device_type == vk::PhysicalDeviceType::INTEGRATED_GPU;

        if !is_unified_memory {
            let has_unified_heap = memory_properties.memory_heaps
                [..memory_properties.memory_heap_count as usize]
                .iter()
                .any(|heap| {
                    heap.flags.contains(vk::MemoryHeapFlags::DEVICE_LOCAL) && heap.size > 0
                });

            if memory_properties.memory_heap_count == 1 && has_unified_heap {
                is_unified_memory = true;
            }
        }

        let mut create_info = vk_mem::AllocatorCreateInfo::new(instance, device, physical_device);

        create_info.flags = vk_mem::AllocatorCreateFlags::BUFFER_DEVICE_ADDRESS
            | vk_mem::AllocatorCreateFlags::EXT_MEMORY_BUDGET;

        let allocator_raw = unsafe {
            Allocator::new(create_info)
                .context("[ApsuAllocator] Failed to create raw VMA Allocator instance")?
        };

        let slot_pool = SlotPool::new(1024, 512, 64);
        let pending_events = Mutex::new(Vec::new());

        let current_timeline_value = Arc::new(AtomicU64::new(0));
        let deferred_deletions = Mutex::new(Vec::new());

        Ok(Self {
            allocator: Arc::new(allocator_raw),
            device: device.clone(),
            physical_device,
            is_unified_memory,
            slot_pool,
            pending_events,
            _device_keeper: None,
            current_timeline_value,
            deferred_deletions,
        })
    }

    pub fn defer_deletion(&self, resource: DeletableResource) {
        let timeline_val = self
            .current_timeline_value
            .load(std::sync::atomic::Ordering::Acquire);
        if let Ok(mut deletions) = self.deferred_deletions.lock() {
            deletions.push(DeferredDeletion {
                resource,
                delete_after_timeline_value: timeline_val,
            });
        }
    }

    pub fn reclaim_resources(&self, current_gpu_value: u64) {
        let mut to_delete = Vec::new();

        if let Ok(mut deletions) = self.deferred_deletions.lock() {
            let mut active = Vec::new();
            for item in std::mem::take(&mut *deletions) {
                if item.delete_after_timeline_value <= current_gpu_value {
                    to_delete.push(item.resource);
                } else {
                    active.push(item);
                }
            }
            *deletions = active;
        }

        for resource in to_delete {
            unsafe {
                match resource {
                    DeletableResource::Buffer(buffer, mut allocation) => {
                        self.allocator.destroy_buffer(buffer, &mut allocation);
                    }
                    DeletableResource::ImageView(view) => {
                        self.device.destroy_image_view(view, None);
                    }
                    DeletableResource::Image(image, mut allocation) => {
                        self.allocator.destroy_image(image, &mut allocation);
                    }
                    DeletableResource::Sampler(sampler) => {
                        self.device.destroy_sampler(sampler, None);
                    }
                    DeletableResource::Slot { slot_type, index } => {
                        if let Err(e) = self.slot_pool.free(slot_type, index) {
                            eprintln!(
                                "[Reclamation] Failed to free slot {:?} index {}: {}",
                                slot_type, index, e
                            );
                        }

                        let event = SlotEvent::Unbind {
                            slot_type,
                            slot_index: index,
                        };
                        self.queue_descriptor_event(event);
                    }
                }
            }
        }
    }

    pub fn query_memory_stats(&self) -> GpuMemoryStats {
        let mut total_vram_bytes = 0u64;
        let mut used_vram_bytes = 0u64;
        let mut available_vram_bytes = 0u64;

        if let Ok(budgets) = self.allocator.get_heap_budgets() {
            for b in budgets {
                let heap_budget = b.budget;
                let heap_usage = b.usage;

                if heap_budget > 0 {
                    total_vram_bytes += heap_budget;
                    used_vram_bytes += heap_usage;
                    available_vram_bytes += heap_budget.saturating_sub(heap_usage);
                }
            }
        }

        GpuMemoryStats {
            total_vram_bytes,
            used_vram_bytes,
            available_vram_bytes,
        }
    }

    pub fn queue_descriptor_event(&self, event: SlotEvent) {
        if let Ok(mut events) = self.pending_events.lock() {
            events.push(event);
        }
    }

    pub fn pull_descriptor_events(&self) -> Vec<SlotEvent> {
        if let Ok(mut events) = self.pending_events.lock() {
            std::mem::take(&mut *events)
        } else {
            Vec::new()
        }
    }
}

impl Drop for ApsuAllocator {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();
        }

        let to_delete = if let Ok(mut deletions) = self.deferred_deletions.lock() {
            std::mem::take(&mut *deletions)
        } else {
            Vec::new()
        };

        for item in to_delete {
            unsafe {
                match item.resource {
                    DeletableResource::Buffer(buffer, mut allocation) => {
                        self.allocator.destroy_buffer(buffer, &mut allocation);
                    }
                    DeletableResource::ImageView(view) => {
                        self.device.destroy_image_view(view, None);
                    }
                    DeletableResource::Image(image, mut allocation) => {
                        self.allocator.destroy_image(image, &mut allocation);
                    }
                    DeletableResource::Sampler(sampler) => {
                        self.device.destroy_sampler(sampler, None);
                    }
                    DeletableResource::Slot { slot_type, index } => {
                        let _ = self.slot_pool.free(slot_type, index);
                    }
                }
            }
        }
    }
}
