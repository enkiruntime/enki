use anyhow::{Context, Result};
use ash::vk;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU32};
use vk_mem::Alloc;

use crate::resources::allocator::ApsuAllocator;
use crate::resources::types::{BufferUsage, MemoryUsage};

static NEXT_BUFFER_ID: AtomicU32 = AtomicU32::new(1);

#[inline]
pub fn next_buffer_id() -> u32 {
    NEXT_BUFFER_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

struct RawGpuBuffer {
    pub buffer: vk::Buffer,
    pub allocation: vk_mem::Allocation,
    pub device_address: u64,
    pub size_in_bytes: u64,
    pub mapped_ptr: *mut std::ffi::c_void,
    allocator_ctx: Arc<ApsuAllocator>,
}

impl RawGpuBuffer {
    fn new(
        allocator_ctx: Arc<ApsuAllocator>,
        size_in_bytes: u64,
        usage: BufferUsage,
        memory_usage: MemoryUsage,
    ) -> Result<Self> {
        let vk_usage = usage
            | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
            | vk::BufferUsageFlags::TRANSFER_SRC
            | vk::BufferUsageFlags::TRANSFER_DST;

        let buffer_info = vk::BufferCreateInfo::default()
            .size(size_in_bytes)
            .usage(vk_usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let (vma_usage, vma_flags) = memory_usage.to_vma(allocator_ctx.is_unified_memory);

        let alloc_info = vk_mem::AllocationCreateInfo {
            usage: vma_usage,
            flags: vma_flags,
            ..Default::default()
        };

        let (buffer, allocation) = unsafe {
            allocator_ctx
                .allocator
                .create_buffer(&buffer_info, &alloc_info)
                .context("[RawGpuBuffer] Failed to allocate VMA Buffer")?
        };

        let address_info = vk::BufferDeviceAddressInfo::default().buffer(buffer);
        let device_address = unsafe {
            allocator_ctx
                .device
                .get_buffer_device_address(&address_info)
        };

        let mut mapped_ptr = std::ptr::null_mut();
        if vma_flags.contains(vk_mem::AllocationCreateFlags::MAPPED) {
            let alloc_info = allocator_ctx.allocator.get_allocation_info(&allocation);
            mapped_ptr = alloc_info.mapped_data;
        }

        Ok(Self {
            buffer,
            allocation,
            device_address,
            size_in_bytes,
            mapped_ptr,
            allocator_ctx,
        })
    }
}

impl Drop for RawGpuBuffer {
    fn drop(&mut self) {
        unsafe {
            let buffer = self.buffer;
            let allocation = std::ptr::read(&self.allocation);

            let resource =
                crate::resources::allocator::DeletableResource::Buffer(buffer, allocation);
            self.allocator_ctx.defer_deletion(resource);
        }
    }
}

pub struct GpuDeviceBuffer {
    raw: RawGpuBuffer,
    pub id: u32,
    pub state: Arc<AtomicU8>,
}

impl GpuDeviceBuffer {
    pub fn new(
        allocator_ctx: Arc<ApsuAllocator>,
        size_in_bytes: u64,
        usage: BufferUsage,
    ) -> Result<Self> {
        let raw = RawGpuBuffer::new(allocator_ctx, size_in_bytes, usage, MemoryUsage::DeviceOnly)?;

        Ok(Self {
            raw,
            id: next_buffer_id(),
            state: Arc::new(AtomicU8::new(0)),
        })
    }

    pub fn buffer(&self) -> vk::Buffer {
        self.raw.buffer
    }
    pub fn device_address(&self) -> u64 {
        self.raw.device_address
    }
    pub fn size_in_bytes(&self) -> u64 {
        self.raw.size_in_bytes
    }
    pub fn slot_index(&self) -> Option<u32> {
        Some(self.id)
    }
}

unsafe impl Send for GpuDeviceBuffer {}
unsafe impl Sync for GpuDeviceBuffer {}

pub struct GpuUploadBuffer {
    raw: RawGpuBuffer,
    pub id: u32,
    pub state: Arc<AtomicU8>,
}

impl GpuUploadBuffer {
    pub fn new(
        allocator_ctx: Arc<ApsuAllocator>,
        size_in_bytes: u64,
        usage: BufferUsage,
    ) -> Result<Self> {
        let raw = RawGpuBuffer::new(allocator_ctx, size_in_bytes, usage, MemoryUsage::Upload)?;

        Ok(Self {
            raw,
            id: next_buffer_id(),
            state: Arc::new(AtomicU8::new(0)),
        })
    }

    pub fn write(&self, data: &[u8]) -> Result<()> {
        self.write_at(0, data)
    }

    pub fn write_at(&self, offset_in_bytes: usize, data: &[u8]) -> Result<()> {
        if self.raw.mapped_ptr.is_null() {
            return Err(anyhow::anyhow!(
                "[GpuUploadBuffer] Mapping failed during allocation"
            ));
        }

        let bytes_to_write = data.len() as u64;
        let offset_bytes_u64 = offset_in_bytes as u64;

        if offset_bytes_u64 + bytes_to_write > self.raw.size_in_bytes {
            return Err(anyhow::anyhow!(
                "[GpuUploadBuffer] Write out of bounds. Capacity: {}, attempted write at offset {} with size {}",
                self.raw.size_in_bytes,
                offset_in_bytes,
                bytes_to_write
            ));
        }

        unsafe {
            let dst_ptr = (self.raw.mapped_ptr as *mut u8).add(offset_in_bytes);
            std::ptr::copy_nonoverlapping(data.as_ptr(), dst_ptr, bytes_to_write as usize);
        }
        Ok(())
    }

    pub fn buffer(&self) -> vk::Buffer {
        self.raw.buffer
    }
    pub fn device_address(&self) -> u64 {
        self.raw.device_address
    }
    pub fn size_in_bytes(&self) -> u64 {
        self.raw.size_in_bytes
    }
    pub fn slot_index(&self) -> Option<u32> {
        Some(self.id)
    }
}

unsafe impl Send for GpuUploadBuffer {}
unsafe impl Sync for GpuUploadBuffer {}

pub struct GpuReadbackBuffer {
    raw: RawGpuBuffer,
    pub id: u32,
    pub state: Arc<AtomicU8>,
}

impl GpuReadbackBuffer {
    pub fn new(
        allocator_ctx: Arc<ApsuAllocator>,
        size_in_bytes: u64,
        usage: BufferUsage,
    ) -> Result<Self> {
        let raw = RawGpuBuffer::new(allocator_ctx, size_in_bytes, usage, MemoryUsage::Download)?;

        Ok(Self {
            raw,
            id: next_buffer_id(),
            state: Arc::new(AtomicU8::new(0)),
        })
    }

    pub fn read(&self, out_data: &mut [u8]) -> Result<()> {
        if self.raw.mapped_ptr.is_null() {
            return Err(anyhow::anyhow!(
                "[GpuReadbackBuffer] Mapping failed during allocation"
            ));
        }

        if out_data.len() as u64 > self.raw.size_in_bytes {
            return Err(anyhow::anyhow!(
                "[GpuReadbackBuffer] Output slice is too small"
            ));
        }

        unsafe {
            std::ptr::copy_nonoverlapping(
                self.raw.mapped_ptr as *const u8,
                out_data.as_mut_ptr(),
                out_data.len(),
            );
        }
        Ok(())
    }

    pub fn buffer(&self) -> vk::Buffer {
        self.raw.buffer
    }
    pub fn device_address(&self) -> u64 {
        self.raw.device_address
    }
    pub fn size_in_bytes(&self) -> u64 {
        self.raw.size_in_bytes
    }
    pub fn slot_index(&self) -> Option<u32> {
        Some(self.id)
    }
}

unsafe impl Send for GpuReadbackBuffer {}
unsafe impl Sync for GpuReadbackBuffer {}

pub struct GpuSharedBuffer {
    raw: RawGpuBuffer,
    pub id: u32,
    pub state: Arc<AtomicU8>,
}

impl GpuSharedBuffer {
    pub fn new(
        allocator_ctx: Arc<ApsuAllocator>,
        size_in_bytes: u64,
        usage: BufferUsage,
    ) -> Result<Self> {
        let raw = RawGpuBuffer::new(allocator_ctx, size_in_bytes, usage, MemoryUsage::ZeroCopy)?;

        Ok(Self {
            raw,
            id: next_buffer_id(),
            state: Arc::new(AtomicU8::new(0)),
        })
    }

    pub fn write(&self, data: &[u8]) -> Result<()> {
        self.write_at(0, data)
    }

    pub fn write_at(&self, offset_in_bytes: usize, data: &[u8]) -> Result<()> {
        if self.raw.mapped_ptr.is_null() {
            return Err(anyhow::anyhow!(
                "[GpuSharedBuffer] Mapping failed during allocation"
            ));
        }

        let bytes_to_write = data.len() as u64;
        let offset_bytes_u64 = offset_in_bytes as u64;

        if offset_bytes_u64 + bytes_to_write > self.raw.size_in_bytes {
            return Err(anyhow::anyhow!(
                "[GpuSharedBuffer] Write out of bounds. Capacity: {}, attempted write at offset {} with size {}",
                self.raw.size_in_bytes,
                offset_in_bytes,
                bytes_to_write
            ));
        }

        unsafe {
            let dst_ptr = (self.raw.mapped_ptr as *mut u8).add(offset_in_bytes);
            std::ptr::copy_nonoverlapping(data.as_ptr(), dst_ptr, bytes_to_write as usize);
        }
        Ok(())
    }

    pub fn read(&self, out_data: &mut [u8]) -> Result<()> {
        if self.raw.mapped_ptr.is_null() {
            return Err(anyhow::anyhow!(
                "[GpuSharedBuffer] Mapping failed during allocation"
            ));
        }

        if out_data.len() as u64 > self.raw.size_in_bytes {
            return Err(anyhow::anyhow!(
                "[GpuSharedBuffer] Output slice is too small"
            ));
        }

        unsafe {
            std::ptr::copy_nonoverlapping(
                self.raw.mapped_ptr as *const u8,
                out_data.as_mut_ptr(),
                out_data.len(),
            );
        }
        Ok(())
    }

    pub fn buffer(&self) -> vk::Buffer {
        self.raw.buffer
    }
    pub fn device_address(&self) -> u64 {
        self.raw.device_address
    }
    pub fn size_in_bytes(&self) -> u64 {
        self.raw.size_in_bytes
    }
    pub fn slot_index(&self) -> Option<u32> {
        Some(self.id)
    }
}

unsafe impl Send for GpuSharedBuffer {}
unsafe impl Sync for GpuSharedBuffer {}
