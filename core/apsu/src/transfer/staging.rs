use std::sync::{Arc, Mutex};
use anyhow::{Result, Context};

use crate::resources::allocator::ApsuAllocator;
use crate::resources::buffer::GpuUploadBuffer;
use crate::resources::types::BufferUsage;

pub struct StagingAllocation {
    pub offset: usize,
    pub size: usize,
}

pub struct StagingRingBuffer {
    buffer: GpuUploadBuffer,
    state: Mutex<RingState>,
}

struct RingState {
    head: usize,
    capacity: usize,
}

impl StagingRingBuffer {
    pub fn new(allocator: Arc<ApsuAllocator>, capacity: usize) -> Result<Self> {
        let buffer = GpuUploadBuffer::new(
            allocator,
            capacity as u64,
            BufferUsage::TRANSFER_SRC,
        ).context("[StagingRingBuffer] Failed to allocate backing upload buffer")?;

        Ok(Self {
            buffer,
            state: Mutex::new(RingState {
                head: 0,
                capacity,
            }),
        })
    }

    pub fn allocate(&self, data: &[u8]) -> Result<StagingAllocation> {
        let size = data.len();

        let aligned_size = (size + 15) & !15;

        let mut state = self.state.lock().unwrap();

        if aligned_size > state.capacity {
            return Err(anyhow::anyhow!(
                "[StagingRingBuffer] Requested allocation size {} exceeds ring buffer capacity {}",
                aligned_size,
                state.capacity
            ));
        }

        if state.head + aligned_size > state.capacity {
            state.head = 0;
        }

        let offset = state.head;
        state.head += aligned_size;

        self.buffer.write_at(offset, data)
            .context("[StagingRingBuffer] Failed to write data to mapped staging memory")?;

        Ok(StagingAllocation {
            offset,
            size,
        })
    }

    pub fn raw_buffer(&self) -> ash::vk::Buffer {
        self.buffer.buffer()
    }

    pub fn capacity(&self) -> usize {
        self.buffer.size_in_bytes() as usize
    }
}