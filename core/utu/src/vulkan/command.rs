use ash::vk;
use anyhow::{Result, Context};

pub struct VulkanCommandPool {
    pub handle: vk::CommandPool,
    device: ash::Device,
}

impl VulkanCommandPool {
    pub fn new(
        device: &ash::Device,
        queue_family_index: u32,
        flags: vk::CommandPoolCreateFlags,
    ) -> Result<Self> {
        let pool_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(queue_family_index)
            .flags(flags);

        let handle = unsafe {
            device.create_command_pool(&pool_info, None)
                .context("[VulkanCommandPool] Failed to create raw Command Pool")?
        };

        Ok(Self {
            handle,
            device: device.clone(),
        })
    }

    pub fn allocate_buffers(
        &self,
        level: vk::CommandBufferLevel,
        count: u32,
    ) -> Result<Vec<vk::CommandBuffer>> {
        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.handle)
            .level(level)
            .command_buffer_count(count);

        let buffers = unsafe {
            self.device.allocate_command_buffers(&alloc_info)
                .context("[VulkanCommandPool] Failed to allocate Command Buffers")?
        };

        Ok(buffers)
    }

    pub fn allocate_buffer(&self, level: vk::CommandBufferLevel) -> Result<vk::CommandBuffer> {
        let buffers = self.allocate_buffers(level, 1)?;
        Ok(buffers[0])
    }

    pub fn free_buffers(&self, buffers: &[vk::CommandBuffer]) {
        unsafe {
            self.device.free_command_buffers(self.handle, buffers);
        }
    }


    pub fn immediate_submit<F, R>(&self, queue: vk::Queue, function: F) -> Result<R>
    where
        F: FnOnce(vk::CommandBuffer) -> Result<R>,
    {
        let cmd = self.allocate_buffer(vk::CommandBufferLevel::PRIMARY)?;

        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            self.device.begin_command_buffer(cmd, &begin_info)
                .context("[VulkanCommandPool] Failed to begin immediate command buffer")?;
        }

        let result = function(cmd);

        unsafe {
            self.device.end_command_buffer(cmd)
                .context("[VulkanCommandPool] Failed to end immediate command buffer")?;
        }

        let return_value = match result {
            Ok(val) => val,
            Err(e) => {
                self.free_buffers(&[cmd]);
                return Err(e);
            }
        };

        let fence_info = vk::FenceCreateInfo::default();
        let fence = unsafe {
            self.device.create_fence(&fence_info, None)
                .context("[VulkanCommandPool] Failed to create sync fence for immediate submit")?
        };

        let command_buffers = [cmd];
        let submit_info = vk::SubmitInfo::default()
            .command_buffers(&command_buffers);

        unsafe {
            self.device.queue_submit(queue, &[submit_info], fence)
                .context("[VulkanCommandPool] Failed to submit immediate command to queue")?;

            self.device.wait_for_fences(&[fence], true, u64::MAX)
                .context("[VulkanCommandPool] Failed to wait for immediate submit fence")?;

            self.device.destroy_fence(fence, None);
            self.free_buffers(&[cmd]);
        }

        Ok(return_value)
    }
}

impl Drop for VulkanCommandPool {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_command_pool(self.handle, None);
        }
    }
}