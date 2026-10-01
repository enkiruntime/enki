use ash::vk;
use std::sync::Arc;

use crate::resources::allocator::ApsuAllocator;
use crate::resources::buffer::GpuDeviceBuffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StackAllocationError {
    pub required_bytes: u64,
    pub available_bytes: u64,
    pub total_bytes: u64,
}

impl std::fmt::Display for StackAllocationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GPU Stack OOM: required {} bytes, but only {} bytes available (total: {} bytes)",
            self.required_bytes, self.available_bytes, self.total_bytes
        )
    }
}

impl std::error::Error for StackAllocationError {}

pub struct GpuStackBuffer {
    pub buffer: GpuDeviceBuffer,
    pub device_address: u64,
    pub size_in_bytes: u64,
}

impl GpuStackBuffer {
    pub fn allocate(
        allocator: Arc<ApsuAllocator>,
        required_bytes: u64,
    ) -> Result<Option<Self>, StackAllocationError> {
        if required_bytes == 0 {
            return Ok(None);
        }

        let stats = allocator.query_memory_stats();

        if !stats.can_allocate(required_bytes) {
            return Err(StackAllocationError {
                required_bytes,
                available_bytes: stats.available_vram_bytes,
                total_bytes: stats.total_vram_bytes,
            });
        }

        let usage = vk::BufferUsageFlags::STORAGE_BUFFER
            | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
            | vk::BufferUsageFlags::TRANSFER_DST;

        let device_buffer =
            GpuDeviceBuffer::new(allocator, required_bytes, usage).map_err(|_| {
                StackAllocationError {
                    required_bytes,
                    available_bytes: stats.available_vram_bytes,
                    total_bytes: stats.total_vram_bytes,
                }
            })?;

        let device_address = device_buffer.device_address();

        Ok(Some(Self {
            buffer: device_buffer,
            device_address,
            size_in_bytes: required_bytes,
        }))
    }

    #[inline(always)]
    pub fn device_address(&self) -> u64 {
        self.device_address
    }

    #[inline(always)]
    pub fn size_in_bytes(&self) -> u64 {
        self.size_in_bytes
    }

    #[inline(always)]
    pub fn vk_buffer(&self) -> vk::Buffer {
        self.buffer.buffer()
    }
}
