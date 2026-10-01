use super::core::GpuVec;
use crate::enki_api::context::active_engine;
use std::mem::MaybeUninit;

impl<T: Copy + Send + Sync + 'static> GpuVec<T> {
    /// Reads a single element from VRAM back to the host CPU at the specified index.
    ///
    /// Automatically synchronizes with the GPU timeline if the buffer is currently in-flight.
    ///
    /// # Diagnostics
    /// Halts execution with diagnostic `error[E02001]` if called inside an active `flow`.
    #[track_caller]
    pub fn get(&self, index: usize) -> Option<T> {
        if index >= self.len {
            return None;
        }

        self.assert_host_readable("get");
        self.wait_idle_if_in_flight();

        let engine = active_engine();
        let element_size = std::mem::size_of::<T>();
        let byte_offset = self.offset as u64 + (index * element_size) as u64;

        let mut out_val = MaybeUninit::<T>::uninit();
        let byte_slice: &mut [u8] = unsafe {
            std::slice::from_raw_parts_mut(out_val.as_mut_ptr() as *mut u8, element_size)
        };

        engine
            .transfer_manager
            .read_buffer(&self._inner, byte_offset, byte_slice)
            .expect("[GpuVec Transfer] Failed to read element from GPU VRAM");

        unsafe { Some(out_val.assume_init()) }
    }

    /// Copies all active elements from VRAM into a newly allocated host `Vec<T>`.
    ///
    /// # Diagnostics
    /// Halts execution with diagnostic `error[E2001]` if called inside an active `flow`.
    #[track_caller]
    pub fn to_vec(&self) -> Vec<T> {
        self.assert_host_readable("to_vec");
        self.wait_idle_if_in_flight();

        if self.len == 0 {
            return Vec::new();
        }

        let engine = active_engine();
        let element_size = std::mem::size_of::<T>();
        let total_bytes = self.len * element_size;

        let mut uninit_vec: Vec<MaybeUninit<T>> = Vec::with_capacity(self.len);
        unsafe {
            uninit_vec.set_len(self.len);
        }

        let byte_slice: &mut [u8] = unsafe {
            std::slice::from_raw_parts_mut(uninit_vec.as_mut_ptr() as *mut u8, total_bytes)
        };

        engine
            .transfer_manager
            .read_buffer(&self._inner, self.offset as u64, byte_slice)
            .expect("[GpuVec Transfer] Failed to read bytes from GPU VRAM");

        unsafe {
            let mut manual_vec = std::mem::ManuallyDrop::new(uninit_vec);
            Vec::from_raw_parts(
                manual_vec.as_mut_ptr() as *mut T,
                manual_vec.len(),
                manual_vec.capacity(),
            )
        }
    }

    /// Sets a single value from the host CPU to the VRAM buffer at the given index.
    ///
    /// # Diagnostics
    /// Halts execution with diagnostic `error[E2001]` if called inside an active `flow`.
    #[track_caller]
    pub fn set(&mut self, index: usize, value: T) -> bool {
        if index >= self.len {
            return false;
        }

        self.copy_from_slice_at(index, std::slice::from_ref(&value));
        true
    }

    /// Overwrites the active contents of this vector with data from a host CPU slice.
    ///
    /// # Panics
    /// Panics if `self.len() != data.len()`.
    ///
    /// # Diagnostics
    /// Halts execution with diagnostic `error[E2001]` if called inside an active `flow`.
    #[track_caller]
    pub fn copy_from_slice(&mut self, data: &[T]) {
        if self.len != data.len() {
            let caller = std::panic::Location::caller();
            let diag = anu::diagnostics::rt::slice_out_of_bounds(caller);
            crate::enki_api::context::errors::emit_and_abort(&diag);
        }

        self.copy_from_slice_at(0, data);
    }

    /// Overwrites a sub-range of VRAM starting at element `offset` with data from a CPU slice.
    ///
    /// # Panics
    /// Panics if `offset + data.len() > self.len()`.
    ///
    /// # Diagnostics
    /// Halts execution with diagnostic `error[E2001]` if called inside an active `flow`.
    #[track_caller]
    pub fn copy_from_slice_at(&mut self, offset: usize, data: &[T]) {
        if offset + data.len() > self.len {
            let caller = std::panic::Location::caller();
            let diag = anu::diagnostics::rt::slice_out_of_bounds(caller);
            crate::enki_api::context::errors::emit_and_abort(&diag);
        }

        self.assert_host_readable("copy_from_slice_at");
        self.wait_idle_if_in_flight();

        if data.is_empty() {
            return;
        }

        let engine = active_engine();
        let element_size = std::mem::size_of::<T>();
        let byte_offset = self.offset as u64 + (offset * element_size) as u64;
        let size_in_bytes = data.len() * element_size;

        let byte_data: &[u8] =
            unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, size_in_bytes) };

        engine
            .transfer_manager
            .write_buffer(&self._inner, byte_offset, byte_data)
            .expect("[GpuVec Transfer] Failed to write slice into GPU VRAM");
    }

    /// Appends all elements from a host CPU slice to the end of the vector in VRAM.
    ///
    /// Reallocates VRAM capacity via exponential growth if necessary.
    ///
    /// # Diagnostics
    /// Halts execution with diagnostic `error[E2001]` if called inside an active `flow`.
    #[track_caller]
    pub fn extend_from_slice(&mut self, data: &[T]) {
        if data.is_empty() {
            return;
        }

        self.assert_host_readable("extend_from_slice");
        self.wait_idle_if_in_flight();

        let old_len = self.len;
        self.reserve(data.len());

        let engine = active_engine();
        let element_size = std::mem::size_of::<T>();
        let byte_offset = self.offset as u64 + (old_len * element_size) as u64;
        let size_in_bytes = data.len() * element_size;

        let byte_data: &[u8] =
            unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, size_in_bytes) };

        engine
            .transfer_manager
            .write_buffer(&self._inner, byte_offset, byte_data)
            .expect("[GpuVec Transfer] Failed to append slice into GPU VRAM");

        self.len += data.len();
    }
}
