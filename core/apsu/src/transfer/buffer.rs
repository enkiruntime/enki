use anyhow::{Result, Context};

use crate::resources::buffer::{GpuDeviceBuffer, GpuUploadBuffer, GpuReadbackBuffer};
use super::manager::GpuTransferManager;

impl GpuTransferManager {
    pub fn write_buffer(
        &self,
        dst_buffer: &GpuDeviceBuffer,
        dst_offset_bytes: u64,
        data: &[u8],
    ) -> Result<()> {
        let data_size_bytes = data.len() as u64;

        if dst_offset_bytes + data_size_bytes > dst_buffer.size_in_bytes() {
            return Err(anyhow::anyhow!(
                "[GpuTransferManager] Write out of bounds. Buffer size: {}, write attempt at: {} with size: {}",
                dst_buffer.size_in_bytes(),
                dst_offset_bytes,
                data_size_bytes
            ));
        }

        if data_size_bytes <= self.staging_buffer.capacity() as u64 {
            let allocation = self.staging_buffer.allocate(data)
                .context("[GpuTransferManager] Failed to allocate from staging ring buffer")?;

            self.execute_copy_command(
                self.staging_buffer.raw_buffer(),
                dst_buffer.buffer(),
                allocation.offset as u64,
                dst_offset_bytes,
                data_size_bytes,
            )?;
        } else {
            let temp_staging = GpuUploadBuffer::new(
                self.allocator_ctx.clone(),
                data_size_bytes,
                crate::resources::types::BufferUsage::TRANSFER_SRC,
            ).context("[GpuTransferManager] Failed to allocate temporary staging buffer")?;

            temp_staging.write(data)?;

            self.execute_copy_command(
                temp_staging.buffer(),
                dst_buffer.buffer(),
                0,
                dst_offset_bytes,
                data_size_bytes,
            )?;
        }

        Ok(())
    }

    pub fn read_buffer(
        &self,
        src_buffer: &GpuDeviceBuffer,
        src_offset_bytes: u64,
        out_data: &mut [u8],
    ) -> Result<()> {
        let data_size_bytes = out_data.len() as u64;

        if src_offset_bytes + data_size_bytes > src_buffer.size_in_bytes() {
            return Err(anyhow::anyhow!(
                "[GpuTransferManager] Read out of bounds. Buffer size: {}, read attempt at: {} with size: {}",
                src_buffer.size_in_bytes(),
                src_offset_bytes,
                data_size_bytes
            ));
        }

        let temp_readback = GpuReadbackBuffer::new(
            self.allocator_ctx.clone(),
            data_size_bytes,
            crate::resources::types::BufferUsage::TRANSFER_DST,
        ).context("[GpuTransferManager] Failed to allocate temporary readback staging buffer")?;

        self.execute_copy_command(
            src_buffer.buffer(),
            temp_readback.buffer(),
            src_offset_bytes,
            0,
            data_size_bytes,
        ).context("[GpuTransferManager] Failed to execute readback copy command")?;

        temp_readback.read(out_data)
            .context("[GpuTransferManager] Failed to read from staging readback buffer")?;

        Ok(())
    }
}