use super::core::GpuVec;
use crate::enki_api::context::active_engine;
use crate::enki_api::resources::slice::range::ResolvedRange;
use crate::enki_api::resources::slice::{Slice, SliceMut};
use std::mem::MaybeUninit;
use std::ops::RangeBounds;

impl<T: Copy + Send + Sync + 'static> GpuVec<T> {
    /// Appends an element to the back of the vector in VRAM, expanding capacity if needed.
    ///
    /// # Diagnostics
    /// Halts execution with diagnostic `error[E2001]` if called inside an active `flow`.
    pub fn push(&mut self, value: T) {
        self.assert_host_readable("push");
        self.wait_idle_if_in_flight();

        if self.len == self.capacity {
            self.reserve(1);
        }

        let engine = active_engine();
        let element_size = std::mem::size_of::<T>();
        let byte_offset = self.offset as u64 + (self.len * element_size) as u64;

        let byte_data: &[u8] =
            unsafe { std::slice::from_raw_parts(&value as *const T as *const u8, element_size) };

        engine
            .transfer_manager
            .write_buffer(&self._inner, byte_offset, byte_data)
            .expect("[GpuVec Ops] Failed to push element into GPU VRAM");

        self.len += 1;
    }

    /// Removes the last element from the vector in VRAM and transfers it back to the host.
    ///
    /// # Diagnostics
    /// Halts execution with diagnostic `error[E2001]` if called inside an active `flow`.
    pub fn pop(&mut self) -> Option<T> {
        self.assert_host_readable("pop");
        self.wait_idle_if_in_flight();

        if self.len == 0 {
            return None;
        }

        self.len -= 1;
        let last_index = self.len;

        let engine = active_engine();
        let element_size = std::mem::size_of::<T>();
        let byte_offset = self.offset as u64 + (last_index * element_size) as u64;

        let mut out_val = MaybeUninit::<T>::uninit();
        let byte_slice: &mut [u8] = unsafe {
            std::slice::from_raw_parts_mut(out_val.as_mut_ptr() as *mut u8, element_size)
        };

        engine
            .transfer_manager
            .read_buffer(&self._inner, byte_offset, byte_slice)
            .expect("[GpuVec Ops] Failed to read popped element from GPU VRAM");

        unsafe { Some(out_val.assume_init()) }
    }

    /// Clears all elements from the vector without deallocating VRAM capacity.
    ///
    /// # Diagnostics
    /// Halts execution with diagnostic `error[E2001]` if called inside an active `flow`.
    #[inline(always)]
    pub fn clear(&mut self) {
        self.assert_host_readable("clear");
        self.wait_idle_if_in_flight();

        self.len = 0;
    }

    /// Shortens the vector, keeping the first `len` elements and dropping the rest.
    pub fn truncate(&mut self, len: usize) {
        self.assert_host_readable("truncate");
        self.wait_idle_if_in_flight();

        if len < self.len {
            self.len = len;
        }
    }

    /// Resizes the vector in-place in VRAM so that `len` equals `new_len`.
    ///
    /// # Diagnostics
    /// Halts execution with diagnostic `error[E2001]` if called inside an active `flow`.
    pub fn resize(&mut self, new_len: usize, value: T) {
        self.assert_host_readable("resize");
        self.wait_idle_if_in_flight();

        if new_len > self.len {
            let additional = new_len - self.len;
            self.reserve(additional);

            let old_len = self.len;
            let host_fill = vec![value; additional];
            let element_size = std::mem::size_of::<T>();
            let byte_offset = self.offset as u64 + (old_len * element_size) as u64;
            let size_in_bytes = additional * element_size;

            let engine = active_engine();
            let byte_data: &[u8] = unsafe {
                std::slice::from_raw_parts(host_fill.as_ptr() as *const u8, size_in_bytes)
            };

            engine
                .transfer_manager
                .write_buffer(&self._inner, byte_offset, byte_data)
                .expect("[GpuVec Ops] Failed to write filled elements during resize");

            self.len = new_len;
        } else {
            self.truncate(new_len);
        }
    }

    /// Overwrites all active elements in the vector with `value`.
    ///
    /// # Diagnostics
    /// Halts execution with diagnostic `error[E2001]` if called inside an active `flow`.
    pub fn fill(&mut self, value: T) {
        if self.len == 0 {
            return;
        }

        self.assert_host_readable("fill");
        self.wait_idle_if_in_flight();

        let host_fill = vec![value; self.len];
        let element_size = std::mem::size_of::<T>();
        let size_in_bytes = self.len * element_size;

        let engine = active_engine();
        let byte_data: &[u8] =
            unsafe { std::slice::from_raw_parts(host_fill.as_ptr() as *const u8, size_in_bytes) };

        engine
            .transfer_manager
            .write_buffer(&self._inner, self.offset as u64, byte_data)
            .expect("[GpuVec Ops] Failed to fill vector in GPU VRAM");
    }
}

impl<T: Copy + Send + Sync + 'static> GpuVec<T> {
    /// Borrows the entire vector as an immutable, read-only [`Slice`].
    #[inline(always)]
    pub fn as_slice(&self) -> Slice<'_, T> {
        self.slice(..)
    }

    /// Borrows a contiguous sub-range of this vector as an immutable [`Slice`].
    #[track_caller]
    pub fn slice<R: RangeBounds<usize>>(&self, range: R) -> Slice<'_, T> {
        let caller = std::panic::Location::caller();
        let resolved = ResolvedRange::resolve(range, self.len, caller);
        let byte_offset = (resolved.start * self.stride as usize) as u64;

        Slice {
            slot_index: self.slot_index,
            offset: self.offset + byte_offset as u32,
            element_offset: resolved.start,
            stride: self.stride,
            len: resolved.count,
            device_address: self.device_address + byte_offset,
            state: self.state.clone(),
            _inner: self._inner.clone(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Borrows a contiguous sub-range of this vector as an exclusive mutable [`SliceMut`].
    #[inline(always)]
    pub fn as_mut_slice(&mut self) -> SliceMut<'_, T> {
        self.slice_mut(..)
    }

    /// Borrows a contiguous sub-range of this vector as an exclusive mutable [`SliceMut`].
    pub fn slice_mut<R: RangeBounds<usize>>(&mut self, range: R) -> SliceMut<'_, T> {
        let caller = std::panic::Location::caller();
        let resolved = ResolvedRange::resolve(range, self.len, caller);
        let byte_offset = (resolved.start * self.stride as usize) as u64;

        SliceMut {
            slot_index: self.slot_index,
            offset: self.offset + byte_offset as u32,
            element_offset: resolved.start,
            stride: self.stride,
            len: resolved.count,
            device_address: self.device_address + byte_offset,
            state: self.state.clone(),
            _inner: self._inner.clone(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Splits this vector into two disjoint mutable sub-slices at the given index.
    #[track_caller]
    #[inline(always)]
    pub fn split_at_mut(&mut self, mid: usize) -> (SliceMut<'_, T>, SliceMut<'_, T>) {
        if mid > self.len {
            let caller = std::panic::Location::caller();
            let diag = anu::diagnostics::rt::slice_out_of_bounds(caller);
            crate::enki_api::context::errors::emit_and_abort(&diag);
        }

        let byte_split = (mid * self.stride as usize) as u64;

        let left = SliceMut {
            slot_index: self.slot_index,
            offset: self.offset,
            element_offset: 0,
            stride: self.stride,
            len: mid,
            device_address: self.device_address,
            state: self.state.clone(),
            _inner: self._inner.clone(),
            _phantom: std::marker::PhantomData,
        };

        let right = SliceMut {
            slot_index: self.slot_index,
            offset: self.offset + byte_split as u32,
            element_offset: mid,
            stride: self.stride,
            len: self.len - mid,
            device_address: self.device_address + byte_split,
            state: self.state.clone(),
            _inner: self._inner.clone(),
            _phantom: std::marker::PhantomData,
        };

        (left, right)
    }

    /// Borrows the vector as a mutable [`SliceMut`] without taking an exclusive `&mut self` borrow.
    ///
    /// # Safety
    /// Bypasses compile-time vector exclusivity on the host. When dispatched to GPU Nams:
    /// - In safe mode (`.run()`), passing this slice across parallel threads (`size_x * size_y * size_z > 1`)
    ///   will halt with diagnostic **`error[E1009]`**.
    /// - In unchecked mode (`.run_unchecked()`), the runtime `BorrowEngine` will still actively
    ///   enforce range disjointness against other active slices of the same buffer (**`error[E1007]`**).
    #[inline(always)]
    pub unsafe fn as_mut_slice_unchecked(&self) -> SliceMut<'_, T> {
        unsafe { self.slice_mut_unchecked(..) }
    }

    /// Borrows a sub-range as a mutable [`SliceMut`] without taking an exclusive `&mut self` borrow.
    ///
    /// # Safety
    /// Bypasses compile-time vector exclusivity on the host. Range disjointness against other
    /// active slices of the same buffer is actively enforced by the runtime `BorrowEngine` (**`error[E1007]`**).
    #[track_caller]
    pub unsafe fn slice_mut_unchecked<R: RangeBounds<usize>>(&self, range: R) -> SliceMut<'_, T> {
        let caller = std::panic::Location::caller();
        let resolved = ResolvedRange::resolve(range, self.len, caller);
        let byte_offset = (resolved.start * self.stride as usize) as u64;

        SliceMut {
            slot_index: self.slot_index,
            offset: self.offset + byte_offset as u32,
            element_offset: resolved.start,
            stride: self.stride,
            len: resolved.count,
            device_address: self.device_address + byte_offset,
            state: self.state.clone(),
            _inner: self._inner.clone(),
            _phantom: std::marker::PhantomData,
        }
    }
}
