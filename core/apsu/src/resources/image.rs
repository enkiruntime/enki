use crate::resources::allocator::ApsuAllocator;
use anyhow::{Context, Result};
use ash::vk;
use std::sync::Arc;
use std::sync::Mutex;
use vk_mem::Alloc;

use crate::slots::handle::SlotHandle;
use crate::slots::pool::{SlotEvent, SlotType};

pub struct GpuImage {
    pub image: vk::Image,
    pub view: vk::ImageView,
    pub allocation: vk_mem::Allocation,
    pub format: vk::Format,
    pub extent: vk::Extent3D,
    pub slot: Option<SlotHandle>,

    pub current_layout: Mutex<vk::ImageLayout>,

    allocator_ctx: Arc<ApsuAllocator>,
}

impl GpuImage {
    pub fn new(
        allocator_ctx: Arc<ApsuAllocator>,
        width: u32,
        height: u32,
        format: vk::Format,
        usage: vk::ImageUsageFlags,
    ) -> Result<Self> {
        let extent = vk::Extent3D {
            width,
            height,
            depth: 1,
        };

        let image_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(extent)
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        let alloc_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::AutoPreferDevice,
            ..Default::default()
        };

        let (image, allocation) = unsafe {
            allocator_ctx
                .allocator
                .create_image(&image_info, &alloc_info)
                .context("[GpuImage] Failed to allocate VMA Image")?
        };

        let view_info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        let view = unsafe {
            allocator_ctx
                .device
                .create_image_view(&view_info, None)
                .context("[GpuImage] Failed to create Image View")?
        };

        let slot_type = if usage.contains(vk::ImageUsageFlags::STORAGE) {
            Some(SlotType::StorageImage)
        } else if usage.contains(vk::ImageUsageFlags::SAMPLED) {
            Some(SlotType::SampledImage)
        } else {
            None
        };

        let mut slot = None;
        if let Some(ty) = slot_type {
            let slot_index = allocator_ctx
                .slot_pool
                .allocate(ty)
                .map_err(|e| anyhow::anyhow!(e))?;

            let handle = SlotHandle::new(ty, slot_index, allocator_ctx.clone());

            let image_layout = if ty == SlotType::SampledImage {
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
            } else {
                vk::ImageLayout::GENERAL
            };

            let event = SlotEvent::BindImage {
                slot_type: ty,
                slot_index,
                image_view: view,
                image_layout,
            };
            allocator_ctx.queue_descriptor_event(event);

            slot = Some(handle);
        }

        let current_layout = Mutex::new(vk::ImageLayout::UNDEFINED);

        Ok(Self {
            image,
            view,
            allocation,
            format,
            extent,
            slot,
            current_layout,
            allocator_ctx,
        })
    }

    pub fn layout(&self) -> vk::ImageLayout {
        *self.current_layout.lock().unwrap()
    }

    pub fn set_layout(&self, layout: vk::ImageLayout) {
        *self.current_layout.lock().unwrap() = layout;
    }

    pub fn slot_index(&self) -> Option<u32> {
        self.slot.as_ref().map(|s| s.index)
    }
}

impl Drop for GpuImage {
    fn drop(&mut self) {
        unsafe {
            self.slot.take();

            let view_resource =
                crate::resources::allocator::DeletableResource::ImageView(self.view);
            let image_resource = crate::resources::allocator::DeletableResource::Image(
                self.image,
                std::ptr::read(&self.allocation),
            );

            self.allocator_ctx.defer_deletion(view_resource);
            self.allocator_ctx.defer_deletion(image_resource);
        }
    }
}
