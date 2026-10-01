use anyhow::{Context, Result};
use ash::vk;
use std::sync::Arc;

use crate::resources::allocator::ApsuAllocator;

use crate::slots::handle::SlotHandle;
use crate::slots::pool::{SlotEvent, SlotType};

pub struct GpuSampler {
    pub sampler: vk::Sampler,
    pub slot: Option<SlotHandle>,

    allocator_ctx: Arc<ApsuAllocator>,
}

impl GpuSampler {
    pub fn new(
        allocator_ctx: Arc<ApsuAllocator>,
        create_info: &vk::SamplerCreateInfo,
    ) -> Result<Self> {
        let sampler = unsafe {
            allocator_ctx
                .device
                .create_sampler(create_info, None)
                .context("[GpuSampler] Failed to create raw Vulkan Sampler")?
        };

        let slot_index = allocator_ctx
            .slot_pool
            .allocate(SlotType::Sampler)
            .map_err(|e| anyhow::anyhow!(e))?;

        let slot_handle = SlotHandle::new(SlotType::Sampler, slot_index, allocator_ctx.clone());

        let event = SlotEvent::BindSampler {
            slot_index,
            sampler,
        };
        allocator_ctx.queue_descriptor_event(event);

        Ok(Self {
            sampler,
            slot: Some(slot_handle),
            allocator_ctx,
        })
    }

    pub fn slot_index(&self) -> Option<u32> {
        self.slot.as_ref().map(|s| s.index)
    }
}

impl Drop for GpuSampler {
    fn drop(&mut self) {
        self.slot.take();

        let resource = crate::resources::allocator::DeletableResource::Sampler(self.sampler);
        self.allocator_ctx.defer_deletion(resource);
    }
}
