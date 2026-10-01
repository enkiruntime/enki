use anyhow::{Context, Result};
use ash::vk;
use std::sync::Arc;

use crate::resources::allocator::ApsuAllocator;
use crate::resources::buffer::{GpuDeviceBuffer, GpuUploadBuffer};
use crate::resources::types::BufferUsage;
use crate::transfer::GpuTransferManager;

pub enum ApsuArenaBuffer {
    Upload(GpuUploadBuffer),
    Device(GpuDeviceBuffer),
}

pub struct ApsuArena {
    buffer: ApsuArenaBuffer,
    size_in_bytes: u64,
    base_address: u64,
}

impl ApsuArena {
    pub fn new_upload(
        allocator: Arc<ApsuAllocator>,
        size_in_bytes: u64,
        usage: BufferUsage,
    ) -> Result<Self> {
        let buffer = GpuUploadBuffer::new(allocator, size_in_bytes, usage)
            .context("[ApsuArena] Failed to allocate Upload Buffer")?;
        let base_address = buffer.device_address();

        Ok(Self {
            buffer: ApsuArenaBuffer::Upload(buffer),
            size_in_bytes,
            base_address,
        })
    }

    pub fn new_device(
        allocator: Arc<ApsuAllocator>,
        size_in_bytes: u64,
        usage: vk::BufferUsageFlags,
    ) -> Result<Self> {
        let buffer = GpuDeviceBuffer::new(allocator, size_in_bytes, usage)
            .context("[ApsuArena] Failed to allocate Device Buffer")?;
        let base_address = buffer.device_address();

        Ok(Self {
            buffer: ApsuArenaBuffer::Device(buffer),
            size_in_bytes,
            base_address,
        })
    }

    #[inline(always)]
    pub fn base_address(&self) -> u64 {
        self.base_address
    }

    #[inline(always)]
    pub fn size_bytes(&self) -> u64 {
        self.size_in_bytes
    }

    #[inline(always)]
    pub fn vk_buffer(&self) -> vk::Buffer {
        match &self.buffer {
            ApsuArenaBuffer::Upload(b) => b.buffer(),
            ApsuArenaBuffer::Device(b) => b.buffer(),
        }
    }

    pub fn write_raw(
        &self,
        offset: usize,
        data: &[u8],
        transfer_manager: Option<&GpuTransferManager>,
    ) -> Result<()> {
        match &self.buffer {
            ApsuArenaBuffer::Upload(upload_buf) => upload_buf
                .write_at(offset, data)
                .context("[ApsuArena] Failed to write directly to mapped upload buffer"),
            ApsuArenaBuffer::Device(device_buf) => {
                let tm = transfer_manager.ok_or_else(|| {
                    anyhow::anyhow!(
                        "[ApsuArena] Transfer manager is required for Device-local writes"
                    )
                })?;
                tm.write_buffer(device_buf, offset as u64, data)
                    .context("[ApsuArena] Failed to write to device buffer via transfer")
            }
        }
    }
}
