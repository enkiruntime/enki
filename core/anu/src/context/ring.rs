use ash::vk;
use anyhow::{Result, Context};
use utu::vulkan::VulkanCommandPool;

pub struct CommandSlot {
    pub cmd: vk::CommandBuffer,
    pub last_submitted_timeline_value: u64,
    pub is_statically_recorded: bool,
}

pub struct TimelineCommandRing {
    pub command_pool: VulkanCommandPool,
    pub slots: Vec<CommandSlot>,
    pub next_slot_idx: usize,
}

impl TimelineCommandRing {
    pub fn new(
        device: &ash::Device,
        queue_family_index: u32,
        capacity: usize,
    ) -> Result<Self> {
        let command_pool = VulkanCommandPool::new(
            device,
            queue_family_index,
            vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
        ).context("[TimelineCommandRing] Failed to create command pool")?;

        let raw_buffers = command_pool.allocate_buffers(
            vk::CommandBufferLevel::PRIMARY,
            capacity as u32,
        ).context("[TimelineCommandRing] Failed to allocate initial command buffers")?;

        let slots = raw_buffers.into_iter()
            .map(|cmd| CommandSlot {
                cmd,
                last_submitted_timeline_value: 0,
                is_statically_recorded: false,
            })
            .collect();

        Ok(Self {
            command_pool,
            slots,
            next_slot_idx: 0,
        })
    }

    pub fn acquire_next_cmd(
        &mut self,
        device: &ash::Device,
        timeline_semaphore: vk::Semaphore,
    ) -> Result<(vk::CommandBuffer, usize)> {
        let slot_idx = self.next_slot_idx % self.slots.len();
        let slot = &mut self.slots[slot_idx];

        let current_gpu_value = unsafe {
            device.get_semaphore_counter_value(timeline_semaphore)
                .context("[TimelineCommandRing] Failed to query timeline semaphore counter value")?
        };

        if current_gpu_value < slot.last_submitted_timeline_value {
            let semaphores = [timeline_semaphore];
            let values = [slot.last_submitted_timeline_value];
            let wait_info = vk::SemaphoreWaitInfo::default()
                .semaphores(&semaphores)
                .values(&values);

            unsafe {
                device.wait_semaphores(&wait_info, u64::MAX)
                    .context("[TimelineCommandRing] Backpressure wait failed")?;
            }
        }

        if !slot.is_statically_recorded {
            unsafe {
                device.reset_command_buffer(slot.cmd, vk::CommandBufferResetFlags::empty())
                    .context("[TimelineCommandRing] Failed to reset command buffer")?;
            }
        }

        self.next_slot_idx += 1;

        Ok((slot.cmd, slot_idx))
    }

    pub fn update_slot_timeline(&mut self, slot_idx: usize, value: u64) {
        if slot_idx < self.slots.len() {
            self.slots[slot_idx].last_submitted_timeline_value = value;
        }
    }
}