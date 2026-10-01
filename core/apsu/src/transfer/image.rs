use anyhow::{Context, Result};
use ash::vk;

use crate::resources::buffer::{GpuReadbackBuffer, GpuUploadBuffer};
use crate::resources::image::GpuImage;

use super::alignment::{calculate_image_copy_params, depad_pixel_data, pad_pixel_data};
use super::manager::GpuTransferManager;

#[allow(clippy::too_many_arguments)]
fn transition_layout(
    device: &ash::Device,
    cmd: vk::CommandBuffer,
    image: vk::Image,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
    aspect_mask: vk::ImageAspectFlags,
    src_stage: vk::PipelineStageFlags2,
    src_access: vk::AccessFlags2,
    dst_stage: vk::PipelineStageFlags2,
    dst_access: vk::AccessFlags2,
) {
    let image_barrier = vk::ImageMemoryBarrier2::default()
        .src_stage_mask(src_stage)
        .src_access_mask(src_access)
        .dst_stage_mask(dst_stage)
        .dst_access_mask(dst_access)
        .old_layout(old_layout)
        .new_layout(new_layout)
        .image(image)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        });

    let dependency_info =
        vk::DependencyInfo::default().image_memory_barriers(std::slice::from_ref(&image_barrier));

    unsafe {
        device.cmd_pipeline_barrier2(cmd, &dependency_info);
    }
}

#[allow(clippy::too_many_arguments)]
impl GpuTransferManager {
    pub fn write_image(
        &self,
        dst_image: &GpuImage,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        bytes_per_pixel: u32,
        data: &[u8],
    ) -> Result<()> {
        let padded_data = pad_pixel_data(data, width, height, bytes_per_pixel, 4);
        let padded_size = padded_data.len() as u64;

        let src_raw: vk::Buffer;
        let src_offset: u64;
        let mut _temp_staging: Option<GpuUploadBuffer> = None;

        if padded_size <= self.staging_buffer.capacity() as u64 {
            let allocation = self.staging_buffer.allocate(&padded_data).context(
                "[GpuTransferManager] Failed to allocate image bytes from staging ring buffer",
            )?;
            src_raw = self.staging_buffer.raw_buffer();
            src_offset = allocation.offset as u64;
        } else {
            let temp = GpuUploadBuffer::new(
                self.allocator_ctx.clone(),
                padded_size,
                crate::resources::types::BufferUsage::TRANSFER_SRC,
            )
            .context("[GpuTransferManager] Failed to allocate temporary upload buffer for image")?;
            temp.write(&padded_data)?;
            src_raw = temp.buffer();
            src_offset = 0;
            _temp_staging = Some(temp);
        }

        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let cmd = unsafe { self.device.allocate_command_buffers(&alloc_info)?[0] };

        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            self.device
                .begin_command_buffer(cmd, &begin_info)
                .context("[GpuTransferManager] Failed to begin recording image write command")?;
        }

        let original_layout = dst_image.layout();

        transition_layout(
            &self.device,
            cmd,
            dst_image.image,
            original_layout,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageAspectFlags::COLOR,
            vk::PipelineStageFlags2::ALL_COMMANDS,
            vk::AccessFlags2::NONE,
            vk::PipelineStageFlags2::TRANSFER,
            vk::AccessFlags2::TRANSFER_WRITE,
        );

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
            .image_offset(vk::Offset3D {
                x: x as i32,
                y: y as i32,
                z: 0,
            })
            .image_extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            });

        unsafe {
            self.device.cmd_copy_buffer_to_image(
                cmd,
                src_raw,
                dst_image.image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                std::slice::from_ref(&copy_region),
            );
        }

        transition_layout(
            &self.device,
            cmd,
            dst_image.image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            original_layout,
            vk::ImageAspectFlags::COLOR,
            vk::PipelineStageFlags2::TRANSFER,
            vk::AccessFlags2::TRANSFER_WRITE,
            vk::PipelineStageFlags2::ALL_COMMANDS,
            vk::AccessFlags2::SHADER_READ | vk::AccessFlags2::SHADER_WRITE,
        );

        unsafe {
            self.device
                .end_command_buffer(cmd)
                .context("[GpuTransferManager] Failed to end image write command recording")?;
        }

        let fence_info = vk::FenceCreateInfo::default();
        let fence = unsafe { self.device.create_fence(&fence_info, None)? };

        let command_buffers = [cmd];
        let submit_info = vk::SubmitInfo::default().command_buffers(&command_buffers);

        unsafe {
            self.device
                .queue_submit(self.queue, &[submit_info], fence)
                .context("[GpuTransferManager] Failed to submit image write commands to queue")?;

            self.device
                .wait_for_fences(&[fence], true, u64::MAX)
                .context("[GpuTransferManager] Failed waiting for image write fence")?;

            self.device.destroy_fence(fence, None);
            self.device.free_command_buffers(self.command_pool, &[cmd]);
        }

        Ok(())
    }

    pub fn read_image(
        &self,
        src_image: &GpuImage,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        bytes_per_pixel: u32,
    ) -> Result<Vec<u8>> {
        let params = calculate_image_copy_params(width, height, bytes_per_pixel, 4);

        let temp_readback = GpuReadbackBuffer::new(
            self.allocator_ctx.clone(),
            params.total_size_bytes as u64,
            crate::resources::types::BufferUsage::TRANSFER_DST,
        )
        .context("[GpuTransferManager] Failed to allocate staging readback buffer for image")?;

        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let cmd = unsafe { self.device.allocate_command_buffers(&alloc_info)?[0] };

        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            self.device
                .begin_command_buffer(cmd, &begin_info)
                .context("[GpuTransferManager] Failed to begin recording image read command")?;
        }

        let original_layout = src_image.layout();

        transition_layout(
            &self.device,
            cmd,
            src_image.image,
            original_layout,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            vk::ImageAspectFlags::COLOR,
            vk::PipelineStageFlags2::ALL_COMMANDS,
            vk::AccessFlags2::NONE,
            vk::PipelineStageFlags2::TRANSFER,
            vk::AccessFlags2::TRANSFER_READ,
        );

        let copy_region = vk::BufferImageCopy::default()
            .buffer_offset(0)
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            })
            .image_offset(vk::Offset3D {
                x: x as i32,
                y: y as i32,
                z: 0,
            })
            .image_extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            });

        unsafe {
            self.device.cmd_copy_image_to_buffer(
                cmd,
                src_image.image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                temp_readback.buffer(),
                std::slice::from_ref(&copy_region),
            );
        }

        transition_layout(
            &self.device,
            cmd,
            src_image.image,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            original_layout,
            vk::ImageAspectFlags::COLOR,
            vk::PipelineStageFlags2::TRANSFER,
            vk::AccessFlags2::TRANSFER_READ,
            vk::PipelineStageFlags2::ALL_COMMANDS,
            vk::AccessFlags2::SHADER_READ | vk::AccessFlags2::SHADER_WRITE,
        );

        unsafe {
            self.device
                .end_command_buffer(cmd)
                .context("[GpuTransferManager] Failed to end image read command recording")?;
        }

        let fence_info = vk::FenceCreateInfo::default();
        let fence = unsafe { self.device.create_fence(&fence_info, None)? };

        let command_buffers = [cmd];
        let submit_info = vk::SubmitInfo::default().command_buffers(&command_buffers);

        unsafe {
            self.device
                .queue_submit(self.queue, &[submit_info], fence)
                .context("[GpuTransferManager] Failed to submit image read commands to queue")?;

            self.device
                .wait_for_fences(&[fence], true, u64::MAX)
                .context("[GpuTransferManager] Failed waiting for image read fence")?;

            self.device.destroy_fence(fence, None);
            self.device.free_command_buffers(self.command_pool, &[cmd]);
        }

        let mut padded_result = vec![0u8; params.total_size_bytes];
        temp_readback.read(&mut padded_result)?;

        let final_unpadded_result =
            depad_pixel_data(&padded_result, width, height, bytes_per_pixel, 4);

        Ok(final_unpadded_result)
    }
}
