use anyhow::{Context, Result};
use ash::vk;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::sync::Arc;
use std::time::Duration;

use crate::vulkan::{VulkanDevice, VulkanInstance};

pub struct GpuWindow {
    surface_loader: ash::khr::surface::Instance,
    surface: vk::SurfaceKHR,

    swapchain_loader: ash::khr::swapchain::Device,
    swapchain: vk::SwapchainKHR,

    pub format: vk::SurfaceFormatKHR,
    pub extent: vk::Extent2D,
    pub images: Vec<vk::Image>,
    pub image_views: Vec<vk::ImageView>,

    pub image_acquired_semaphores: Vec<vk::Semaphore>,
    pub render_finished_semaphores: Vec<vk::Semaphore>,
    pub in_flight_fences: Vec<vk::Fence>,

    current_frame: usize,
    max_frames_in_flight: usize,
    last_acquired_image: usize,

    device: Arc<VulkanDevice>,
}

impl GpuWindow {
    pub fn new(
        instance: &VulkanInstance,
        physical_device: vk::PhysicalDevice,
        device: &Arc<VulkanDevice>,
        window: &impl HasWindowHandle,
        display: &impl HasDisplayHandle,
        width: u32,
        height: u32,
        max_frames_in_flight: usize,
    ) -> Result<Self> {
        let display_handle = display
            .display_handle()
            .map_err(|e| anyhow::anyhow!("[GpuWindow] Failed to get RawDisplayHandle: {:?}", e))?
            .as_raw();

        let window_handle = window
            .window_handle()
            .map_err(|e| anyhow::anyhow!("[GpuWindow] Failed to get RawWindowHandle: {:?}", e))?
            .as_raw();

        let surface = unsafe {
            ash_window::create_surface(
                &instance.entry,
                &instance.instance,
                display_handle,
                window_handle,
                None,
            )
            .context("[GpuWindow] Failed to create Vulkan Surface")?
        };

        let surface_loader = ash::khr::surface::Instance::new(&instance.entry, &instance.instance);
        let swapchain_loader =
            ash::khr::swapchain::Device::new(&instance.instance, &device.logical_device);

        let mut window_ctx = Self {
            surface_loader,
            surface,
            swapchain_loader,
            swapchain: vk::SwapchainKHR::null(),
            format: vk::SurfaceFormatKHR::default(),
            extent: vk::Extent2D::default(),
            images: Vec::new(),
            image_views: Vec::new(),
            image_acquired_semaphores: Vec::new(),
            render_finished_semaphores: Vec::new(),
            in_flight_fences: Vec::new(),
            current_frame: 0,
            last_acquired_image: 0,
            max_frames_in_flight,
            device: device.clone(),
        };

        let (swapchain, format, extent, images, image_views) = window_ctx
            .create_swapchain_and_views_raw(
                physical_device,
                vk::SwapchainKHR::null(),
                width,
                height,
            )?;

        window_ctx.swapchain = swapchain;
        window_ctx.format = format;
        window_ctx.extent = extent;
        window_ctx.images = images;
        window_ctx.image_views = image_views;
        window_ctx.max_frames_in_flight = window_ctx.images.len();

        let semaphore_info = vk::SemaphoreCreateInfo::default();
        let fence_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);

        for _ in 0..window_ctx.max_frames_in_flight {
            let acq_sem = unsafe {
                device
                    .logical_device
                    .create_semaphore(&semaphore_info, None)
                    .context("[GpuWindow] Failed to create image acquired semaphore")?
            };
            let render_sem = unsafe {
                device
                    .logical_device
                    .create_semaphore(&semaphore_info, None)
                    .context("[GpuWindow] Failed to create render finished semaphore")?
            };
            let inflight_fence = unsafe {
                device
                    .logical_device
                    .create_fence(&fence_info, None)
                    .context("[GpuWindow] Failed to create in flight fence")?
            };

            window_ctx.image_acquired_semaphores.push(acq_sem);
            window_ctx.render_finished_semaphores.push(render_sem);
            window_ctx.in_flight_fences.push(inflight_fence);
        }

        Ok(window_ctx)
    }

    pub fn recreate(
        &mut self,
        physical_device: vk::PhysicalDevice,
        new_width: u32,
        new_height: u32,
    ) -> Result<()> {
        unsafe {
            self.device
                .device_wait_idle()
                .context("[GpuWindow] Failed wait idle before recreate swapchain")?;

            for &view in &self.image_views {
                self.device.destroy_image_view(view, None);
            }

            let old_swapchain = self.swapchain;

            let (new_swapchain, new_format, new_extent, new_images, new_image_views) = self
                .create_swapchain_and_views_raw(
                    physical_device,
                    old_swapchain,
                    new_width,
                    new_height,
                )?;

            self.swapchain_loader.destroy_swapchain(old_swapchain, None);

            self.swapchain = new_swapchain;
            self.format = new_format;
            self.extent = new_extent;
            self.images = new_images;
            self.image_views = new_image_views;
        }

        Ok(())
    }

    fn create_swapchain_and_views_raw(
        &self,
        physical_device: vk::PhysicalDevice,
        old_swapchain: vk::SwapchainKHR,
        width: u32,
        height: u32,
    ) -> Result<(
        vk::SwapchainKHR,
        vk::SurfaceFormatKHR,
        vk::Extent2D,
        Vec<vk::Image>,
        Vec<vk::ImageView>,
    )> {
        unsafe {
            let capabilities = self
                .surface_loader
                .get_physical_device_surface_capabilities(physical_device, self.surface)
                .context("[GpuWindow] Failed to get surface capabilities")?;

            let formats = self
                .surface_loader
                .get_physical_device_surface_formats(physical_device, self.surface)
                .context("[GpuWindow] Failed to get surface formats")?;

            let present_modes = self
                .surface_loader
                .get_physical_device_surface_present_modes(physical_device, self.surface)
                .context("[GpuWindow] Failed to get surface present modes")?;

            let format = formats
                .iter()
                .find(|f| {
                    f.format == vk::Format::B8G8R8A8_SRGB
                        && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
                })
                .cloned()
                .unwrap_or_else(|| formats[0]);

            let present_mode = present_modes
                .iter()
                .cloned()
                .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
                .unwrap_or(vk::PresentModeKHR::FIFO);

            let extent = if capabilities.current_extent.width != u32::MAX {
                capabilities.current_extent
            } else {
                vk::Extent2D {
                    width: width.clamp(
                        capabilities.min_image_extent.width,
                        capabilities.max_image_extent.width,
                    ),
                    height: height.clamp(
                        capabilities.min_image_extent.height,
                        capabilities.max_image_extent.height,
                    ),
                }
            };

            let mut image_count = capabilities.min_image_count + 1;
            if capabilities.max_image_count > 0 && image_count > capabilities.max_image_count {
                image_count = capabilities.max_image_count;
            }

            let composite_alpha = [
                vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED,
                vk::CompositeAlphaFlagsKHR::POST_MULTIPLIED,
                vk::CompositeAlphaFlagsKHR::INHERIT,
                vk::CompositeAlphaFlagsKHR::OPAQUE,
            ]
            .into_iter()
            .find(|&c| capabilities.supported_composite_alpha.contains(c))
            .unwrap_or(vk::CompositeAlphaFlagsKHR::OPAQUE);

            let swapchain_info = vk::SwapchainCreateInfoKHR::default()
                .surface(self.surface)
                .min_image_count(image_count)
                .image_format(format.format)
                .image_color_space(format.color_space)
                .image_extent(extent)
                .image_array_layers(1)
                .image_usage(
                    vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST,
                )
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                .pre_transform(capabilities.current_transform)
                .composite_alpha(composite_alpha)
                .present_mode(present_mode)
                .clipped(true)
                .old_swapchain(old_swapchain);

            let swapchain = self
                .swapchain_loader
                .create_swapchain(&swapchain_info, None)
                .context("[GpuWindow] Failed to create Vulkan Swapchain")?;

            let images = self
                .swapchain_loader
                .get_swapchain_images(swapchain)
                .context("[GpuWindow] Failed to get swapchain images")?;

            let mut image_views = Vec::with_capacity(images.len());
            for &image in &images {
                let view_info = vk::ImageViewCreateInfo::default()
                    .image(image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(format.format)
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    });

                let view = self
                    .device
                    .create_image_view(&view_info, None)
                    .context("[GpuWindow] Failed to create ImageView for swapchain image")?;

                image_views.push(view);
            }

            Ok((swapchain, format, extent, images, image_views))
        }
    }

    pub fn acquire_next_image(&mut self, timeout: Duration) -> Result<(u32, bool)> {
        unsafe {
            let current_fence = self.in_flight_fences[self.current_frame];
            self.device
                .wait_for_fences(&[current_fence], true, u64::MAX)
                .context("[GpuWindow] Failed to wait for in flight fence")?;

            self.device
                .reset_fences(&[current_fence])
                .context("[GpuWindow] Failed to reset in flight fence")?;

            let current_semaphore = self.image_acquired_semaphores[self.current_frame];

            let result = self.swapchain_loader.acquire_next_image(
                self.swapchain,
                timeout.as_nanos() as u64,
                current_semaphore,
                vk::Fence::null(),
            );

            match result {
                Ok((idx, suboptimal)) => {
                    self.last_acquired_image = idx as usize;
                    Ok((idx, suboptimal))
                }
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => Ok((0, true)),
                Err(e) => Err(anyhow::anyhow!(e)
                    .context("[GpuWindow] Failed to acquire next swapchain image")),
            }
        }
    }

    pub fn cmd_copy_buffer_to_image(
        &self,
        cmd: vk::CommandBuffer,
        image_index: u32,
        src_buffer: vk::Buffer,
        src_offset: u64,
        width: u32,
        height: u32,
    ) {
        let image = self.images[image_index as usize];

        let barrier_to_transfer = vk::ImageMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
            .src_access_mask(vk::AccessFlags2::MEMORY_WRITE | vk::AccessFlags2::MEMORY_READ)
            .dst_stage_mask(vk::PipelineStageFlags2::TRANSFER)
            .dst_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .image(image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        let dep_info = vk::DependencyInfo::default()
            .image_memory_barriers(std::slice::from_ref(&barrier_to_transfer));

        unsafe {
            self.device
                .logical_device
                .cmd_pipeline_barrier2(cmd, &dep_info);
        }

        let copy_region = vk::BufferImageCopy::default()
            .buffer_offset(src_offset)
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            })
            .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
            .image_extent(vk::Extent3D {
                width: width.min(self.extent.width),
                height: height.min(self.extent.height),
                depth: 1,
            });

        unsafe {
            self.device.logical_device.cmd_copy_buffer_to_image(
                cmd,
                src_buffer,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                std::slice::from_ref(&copy_region),
            );
        }

        let barrier_to_present = vk::ImageMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::TRANSFER)
            .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
            .dst_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
            .dst_access_mask(vk::AccessFlags2::NONE)
            .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .image(image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        let dep_info = vk::DependencyInfo::default()
            .image_memory_barriers(std::slice::from_ref(&barrier_to_present));

        unsafe {
            self.device
                .logical_device
                .cmd_pipeline_barrier2(cmd, &dep_info);
        }
    }

    pub fn present_image(&mut self, queue: vk::Queue, image_index: u32) -> Result<bool> {
        unsafe {
            let current_semaphore = self.render_finished_semaphores[image_index as usize];
            let swapchains = [self.swapchain];
            let image_indices = [image_index];

            let present_info = vk::PresentInfoKHR::default()
                .wait_semaphores(std::slice::from_ref(&current_semaphore))
                .swapchains(&swapchains)
                .image_indices(&image_indices);

            let result = self.swapchain_loader.queue_present(queue, &present_info);

            self.current_frame = (self.current_frame + 1) % self.max_frames_in_flight;

            match result {
                Ok(suboptimal) => Ok(suboptimal),
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) | Err(vk::Result::SUBOPTIMAL_KHR) => {
                    Ok(true)
                }
                Err(e) => {
                    Err(anyhow::anyhow!(e).context("[GpuWindow] Failed to present swapchain image"))
                }
            }
        }
    }

    pub fn current_image_acquired_semaphore(&self) -> vk::Semaphore {
        self.image_acquired_semaphores[self.current_frame]
    }

    pub fn current_render_finished_semaphore(&self) -> vk::Semaphore {
        self.render_finished_semaphores[self.last_acquired_image]
    }

    pub fn current_in_flight_fence(&self) -> vk::Fence {
        self.in_flight_fences[self.current_frame]
    }
}

impl Drop for GpuWindow {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();

            for &view in &self.image_views {
                self.device.destroy_image_view(view, None);
            }

            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);

            for &sem in &self.image_acquired_semaphores {
                self.device.destroy_semaphore(sem, None);
            }

            for &sem in &self.render_finished_semaphores {
                self.device.destroy_semaphore(sem, None);
            }

            for &fence in &self.in_flight_fences {
                self.device.destroy_fence(fence, None);
            }

            self.surface_loader.destroy_surface(self.surface, None);
        }
    }
}
