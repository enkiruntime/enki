use ash::vk;
use anyhow::{Result, Context};
use std::time::Duration;

pub struct VulkanFence {
    pub handle: vk::Fence,
    device: ash::Device,
}

impl VulkanFence {
    pub fn new(device: &ash::Device, signaled: bool) -> Result<Self> {
        let mut flags = vk::FenceCreateFlags::empty();
        if signaled {
            flags |= vk::FenceCreateFlags::SIGNALED;
        }

        let fence_info = vk::FenceCreateInfo::default().flags(flags);
        let handle = unsafe {
            device.create_fence(&fence_info, None)
                .context("[VulkanFence] Failed to create raw Fence")?
        };

        Ok(Self {
            handle,
            device: device.clone(),
        })
    }

    pub fn wait(&self, timeout: Duration) -> Result<()> {
        let timeout_ns = timeout.as_nanos() as u64;
        unsafe {
            self.device.wait_for_fences(&[self.handle], true, timeout_ns)
                .context("[VulkanFence] Failed to wait for Fence")?;
        }
        Ok(())
    }

    pub fn reset(&self) -> Result<()> {
        unsafe {
            self.device.reset_fences(&[self.handle])
                .context("[VulkanFence] Failed to reset Fence")?;
        }
        Ok(())
    }

    pub fn is_signaled(&self) -> Result<bool> {
        unsafe {
            let status = self.device.get_fence_status(self.handle);
            match status {
                Ok(true) => Ok(true),
                Ok(false) => Ok(false),
                Err(e) => Err(anyhow::anyhow!(e).context("[VulkanFence] Failed to query Fence status")),
            }
        }
    }
}

impl Drop for VulkanFence {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_fence(self.handle, None);
        }
    }
}

pub struct VulkanSemaphore {
    pub handle: vk::Semaphore,
    pub is_timeline: bool,
    device: ash::Device,
}

impl VulkanSemaphore {
    pub fn new_binary(device: &ash::Device) -> Result<Self> {
        let sem_info = vk::SemaphoreCreateInfo::default();
        let handle = unsafe {
            device.create_semaphore(&sem_info, None)
                .context("[VulkanSemaphore] Failed to create raw Binary Semaphore")?
        };

        Ok(Self {
            handle,
            is_timeline: false,
            device: device.clone(),
        })
    }

    pub fn new_timeline(device: &ash::Device, initial_value: u64) -> Result<Self> {
        let mut type_info = vk::SemaphoreTypeCreateInfo::default()
            .semaphore_type(vk::SemaphoreType::TIMELINE)
            .initial_value(initial_value);

        let sem_info = vk::SemaphoreCreateInfo::default()
            .push_next(&mut type_info);

        let handle = unsafe {
            device.create_semaphore(&sem_info, None)
                .context("[VulkanSemaphore] Failed to create raw Timeline Semaphore")?
        };

        Ok(Self {
            handle,
            is_timeline: true,
            device: device.clone(),
        })
    }

    pub fn get_timeline_value(&self) -> Result<u64> {
        if !self.is_timeline {
            return Err(anyhow::anyhow!("[VulkanSemaphore] Cannot get counter value of a Binary Semaphore"));
        }
        let value = unsafe {
            self.device.get_semaphore_counter_value(self.handle)
                .context("[VulkanSemaphore] Failed to query Timeline Semaphore value")?
        };
        Ok(value)
    }

    pub fn wait_timeline(&self, target_value: u64, timeout: Duration) -> Result<()> {
        if !self.is_timeline {
            return Err(anyhow::anyhow!("[VulkanSemaphore] Cannot wait on a Binary Semaphore using wait_timeline"));
        }
        let timeout_ns = timeout.as_nanos() as u64;
        let semaphores = [self.handle];
        let values = [target_value];

        let wait_info = vk::SemaphoreWaitInfo::default()
            .semaphores(&semaphores)
            .values(&values);

        unsafe {
            self.device.wait_semaphores(&wait_info, timeout_ns)
                .context("[VulkanSemaphore] Failed during Timeline Semaphore wait")?;
        }
        Ok(())
    }

    pub fn signal_timeline(&self, value: u64) -> Result<()> {
        if !self.is_timeline {
            return Err(anyhow::anyhow!("[VulkanSemaphore] Cannot signal a Binary Semaphore using signal_timeline"));
        }
        let signal_info = vk::SemaphoreSignalInfo::default()
            .semaphore(self.handle)
            .value(value);

        unsafe {
            self.device.signal_semaphore(&signal_info)
                .context("[VulkanSemaphore] Failed to signal Timeline Semaphore")?;
        }
        Ok(())
    }
}

impl Drop for VulkanSemaphore {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_semaphore(self.handle, None);
        }
    }
}