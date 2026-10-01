use super::core::GpuVec;
use super::state::BufferPhase;
use crate::enki_api::context::active_engine;
use std::marker::PhantomData;

impl<T: Copy + Send + Sync + 'static> GpuVec<T> {
    pub(crate) fn wait_idle_if_in_flight(&self) {
        if let BufferPhase::InFlight { target_timeline } = self.current_phase() {
            let engine = active_engine();
            engine
                .timeline_semaphore
                .wait_timeline(target_timeline, std::time::Duration::from_secs(5))
                .expect("[GpuVec Lifecycle] Timeout waiting for GPU to finish in-flight work");
        }
    }

    pub(crate) fn reallocate(&mut self, new_capacity: usize) {
        self.assert_host_readable("reallocate");
        self.wait_idle_if_in_flight();

        assert!(
            new_capacity >= self.len,
            "[GpuVec Lifecycle] New capacity must be greater than or equal to current len"
        );

        let (new_buffer, new_bda, new_slot, new_size_bytes) = Self::allocate_raw(new_capacity);

        if self.len > 0 {
            let engine = active_engine();
            let bytes_to_copy = (self.len * self.stride as usize) as u64;

            engine
                .transfer_manager
                .execute_copy_command(
                    self._inner.buffer(),
                    new_buffer.buffer(),
                    0,
                    0,
                    bytes_to_copy,
                )
                .expect("[GpuVec Lifecycle] Failed to copy elements during VRAM reallocation");
        }

        self.slot_index = new_slot;
        self.capacity = new_capacity;
        self.device_address = new_bda;
        self.size_in_bytes = new_size_bytes;
        self.state = new_buffer.state.clone();
        self._inner = new_buffer;
    }

    /// Reserves capacity for at least `additional` more elements to be inserted.
    ///
    /// # Diagnostics
    /// Halts execution with diagnostic `error[E2001]` if called inside an active `flow`.
    pub fn reserve(&mut self, additional: usize) {
        let required_capacity = self
            .len
            .checked_add(additional)
            .expect("[GpuVec] Capacity overflow");

        if required_capacity > self.capacity {
            let new_capacity = (self.capacity * 2).max(required_capacity).max(4);
            self.reallocate(new_capacity);
        }
    }
}

impl<T: Copy + Send + Sync + 'static> Clone for GpuVec<T> {
    /// Creates an independent physical duplicate of the vector in VRAM.
    ///
    /// Allocates a new buffer with a distinct BDA and unique engine slot index,
    /// executing a direct VRAM-to-VRAM copy command.
    fn clone(&self) -> Self {
        self.assert_host_readable("clone");

        self.wait_idle_if_in_flight();

        let (new_buffer, new_bda, new_slot, new_size_bytes) = Self::allocate_raw(self.capacity);

        if self.len > 0 {
            let engine = active_engine();
            let bytes_to_copy = (self.len * self.stride as usize) as u64;

            engine
                .transfer_manager
                .execute_copy_command(
                    self._inner.buffer(),
                    new_buffer.buffer(),
                    0,
                    0,
                    bytes_to_copy,
                )
                .expect("[GpuVec Lifecycle] VRAM-to-VRAM copy failed during clone");
        }

        Self {
            slot_index: new_slot,
            offset: self.offset,
            stride: self.stride,
            len: self.len,
            capacity: self.capacity,
            device_address: new_bda,
            size_in_bytes: new_size_bytes,
            state: new_buffer.state.clone(),
            _inner: new_buffer,
            _phantom: PhantomData,
        }
    }
}
