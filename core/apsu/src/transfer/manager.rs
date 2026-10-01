use ash::vk;
use anyhow::{Result, Context};
use std::sync::{Arc, Mutex};

use crate::resources::allocator::ApsuAllocator;
use super::staging::StagingRingBuffer;

pub struct GpuTransferManager {
    pub allocator_ctx: Arc<ApsuAllocator>,
    pub device: ash::Device,
    pub queue: vk::Queue,

    submit_lock: Mutex<()>,

    pub(crate) command_pool: vk::CommandPool,
    pub staging_buffer: StagingRingBuffer,
}

impl GpuTransferManager {
    pub fn new(
        allocator_ctx: Arc<ApsuAllocator>,
        queue: vk::Queue,
        command_pool: vk::CommandPool,
        staging_size_in_bytes: usize,
    ) -> Result<Self> {
        let device = allocator_ctx.device.clone();

        let staging_buffer = StagingRingBuffer::new(
            allocator_ctx.clone(),
            staging_size_in_bytes,
        ).context("[GpuTransferManager] Failed to allocate staging ring buffer")?;

        Ok(Self {
            allocator_ctx,
            device,
            queue,
            submit_lock: Mutex::new(()),
            command_pool,
            staging_buffer,
        })
    }

    pub fn execute_copy_command(
        &self,
        src_raw: vk::Buffer,
        dst_raw: vk::Buffer,
        src_offset: u64,
        dst_offset: u64,
        size: u64,
    ) -> Result<()> {
        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let cmd = unsafe {
            self.device.allocate_command_buffers(&alloc_info)
                .context("[GpuTransferManager] Failed to allocate temporary command buffer")?[0]
        };

        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            self.device.begin_command_buffer(cmd, &begin_info)
                .context("[GpuTransferManager] Failed to begin recording transfer command")?;

            let copy_region = vk::BufferCopy::default()
                .src_offset(src_offset)
                .dst_offset(dst_offset)
                .size(size);

            self.device.cmd_copy_buffer(cmd, src_raw, dst_raw, &[copy_region]);

            self.device.end_command_buffer(cmd)
                .context("[GpuTransferManager] Failed to end recording transfer command")?;
        }

        let fence_info = vk::FenceCreateInfo::default();
        let fence = unsafe {
            self.device.create_fence(&fence_info, None)
                .context("[GpuTransferManager] Failed to create sync fence")?
        };

        let command_buffers = [cmd];
        let submit_info = vk::SubmitInfo::default()
            .command_buffers(&command_buffers);

        {
            let _lock = self.submit_lock.lock().unwrap();
            unsafe {
                self.device.queue_submit(self.queue, &[submit_info], fence)
                    .context("[GpuTransferManager] Failed to submit copy command")?;
            }
        }

        unsafe {
            self.device.wait_for_fences(&[fence], true, u64::MAX)
                .context("[GpuTransferManager] Failed waiting for transfer fence")?;

            self.device.destroy_fence(fence, None);
            self.device.free_command_buffers(self.command_pool, &[cmd]);
        }

        Ok(())
    }
}