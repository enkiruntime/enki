use std::marker::PhantomData;
use std::ops::{Range, RangeBounds};
use std::sync::Arc;
use std::sync::atomic::AtomicU8;

use crate::enki_api::context::active_engine;
use crate::enki_api::context::ambient::{is_inside_active_flow, resolve_user_caller};
use crate::enki_api::context::errors::emit_and_abort;

use super::range::ResolvedRange;
use super::ro::Slice;
use crate::enki_api::resources::GpuVec;
use apsu::GpuDeviceBuffer;

/// An exclusive, mutable borrowed view into a contiguous sub-range of GPU memory.
///
/// Enforces Rust's exclusive reference semantics over a VRAM sub-range.
/// Does not implement [`Clone`] to guarantee single-writer exclusivity.
///
/// # Nam Dispatch & Safety Policies
/// Passing `&mut SliceMut<T>` to a `#[nam]` function binds as a global read-write slice **`&mut [T]`**.
///
/// - **Parallel Safe Mode (`.run()`):** If the execution domain has more than one thread
///   (`size_x * size_y * size_z > 1`), passing a mutable slice is strictly forbidden and halts
///   execution with diagnostic **`error[E1009]`** because unconstrained concurrent writes
///   cannot be statically proven race-free.
/// - **Unchecked Dispatch (`.run_unchecked()`):** Required to pass `SliceMut` across parallel
///   threads when writes are manually coordinated (e.g., via atomic slot indexing or disjoint offsets).
/// - **Spatial Disjointness:** In all dispatch modes, if multiple slices from the same root
///   buffer are passed where at least one is mutable, any overlapping range halts execution
///   with diagnostic **`error[E1007]`**.
pub struct SliceMut<'a, T: Copy + Send + Sync + 'static> {
    pub(crate) slot_index: u32,
    pub(crate) offset: u32,
    pub(crate) element_offset: usize,
    pub(crate) stride: u32,
    pub(crate) len: usize,
    pub(crate) device_address: u64,
    pub(crate) state: Arc<AtomicU8>,
    pub(crate) _inner: Arc<GpuDeviceBuffer>,
    pub(crate) _phantom: PhantomData<&'a mut T>,
}

impl<'a, T: Copy + Send + Sync + 'static> SliceMut<'a, T> {
    /// Returns the number of elements inside this slice.
    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the slice contains no elements.
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the unique identity of the underlying parent buffer allocation in VRAM.
    #[inline(always)]
    pub const fn root_id(&self) -> usize {
        self.slot_index as usize
    }

    /// Returns the concrete element range `[start..end)` relative to the parent buffer.
    #[inline(always)]
    pub fn element_range(&self) -> Range<usize> {
        self.element_offset..(self.element_offset + self.len)
    }

    /// Returns the 64-bit GPU Buffer Device Address (BDA) pointing to the start of this sub-range.
    #[inline(always)]
    pub const fn device_address(&self) -> u64 {
        self.device_address
    }

    /// Returns the internal engine slot index of the underlying physical buffer.
    #[inline(always)]
    pub const fn slot_index(&self) -> u32 {
        self.slot_index
    }

    /// Returns the size in bytes of a single element `T`.
    #[inline(always)]
    pub const fn stride(&self) -> usize {
        self.stride as usize
    }

    #[track_caller]
    fn assert_host_writable(&self, operation: &'static str) {
        if is_inside_active_flow() {
            let raw_caller = std::panic::Location::caller();
            let caller = resolve_user_caller(raw_caller);
            let diag = anu::diagnostics::rt::phase_violation(operation, caller);
            emit_and_abort(&diag);
        }
    }

    /// Splits this mutable slice into two disjoint mutable sub-slices at index `mid`.
    ///
    /// Guarantees that the two returned slices are mathematically non-overlapping.
    ///
    /// # Panics
    /// Panics if `mid > self.len()`.
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
            element_offset: self.element_offset,
            stride: self.stride,
            len: mid,
            device_address: self.device_address,
            state: self.state.clone(),
            _inner: self._inner.clone(),
            _phantom: PhantomData,
        };

        let right = SliceMut {
            slot_index: self.slot_index,
            offset: self.offset + byte_split as u32,
            element_offset: self.element_offset + mid,
            stride: self.stride,
            len: self.len - mid,
            device_address: self.device_address + byte_split,
            state: self.state.clone(),
            _inner: self._inner.clone(),
            _phantom: PhantomData,
        };

        (left, right)
    }
    /// Reborrows a smaller contiguous mutable sub-range from this slice.
    ///
    /// # Diagnostics
    /// Halts execution with diagnostic **`error[E2005]`** if `range` exceeds the slice bounds.
    #[track_caller]
    pub fn slice_mut<R: RangeBounds<usize>>(&mut self, range: R) -> SliceMut<'_, T> {
        let caller = std::panic::Location::caller();
        let resolved = ResolvedRange::resolve(range, self.len, caller);
        let byte_offset = (resolved.start * self.stride as usize) as u64;

        SliceMut {
            slot_index: self.slot_index,
            offset: self.offset + byte_offset as u32,
            element_offset: self.element_offset + resolved.start,
            stride: self.stride,
            len: resolved.count,
            device_address: self.device_address + byte_offset,
            state: self.state.clone(),
            _inner: self._inner.clone(),
            _phantom: PhantomData,
        }
    }

    /// Reborrows this mutable slice as an immutable, read-only [`Slice`].
    #[inline(always)]
    pub fn as_slice(&self) -> Slice<'_, T> {
        Slice {
            slot_index: self.slot_index,
            offset: self.offset,
            element_offset: self.element_offset,
            stride: self.stride,
            len: self.len,
            device_address: self.device_address,
            state: self.state.clone(),
            _inner: self._inner.clone(),
            _phantom: PhantomData,
        }
    }

    /// Consumes this mutable slice and converts it into an immutable [`Slice`].
    #[inline(always)]
    pub fn into_slice(self) -> Slice<'a, T> {
        Slice {
            slot_index: self.slot_index,
            offset: self.offset,
            element_offset: self.element_offset,
            stride: self.stride,
            len: self.len,
            device_address: self.device_address,
            state: self.state.clone(),
            _inner: self._inner,
            _phantom: PhantomData,
        }
    }

    /// Sets a single value from the host CPU into the GPU slice at the specified index.
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

    /// Overwrites the entire contents of this GPU slice with data from a host CPU slice.
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

    /// Overwrites a sub-range of this GPU slice starting at element `offset` with host data.
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

        self.assert_host_writable("write to `SliceMut`");

        if data.is_empty() {
            return;
        }

        let engine = active_engine();
        let target_timeline = engine
            .timeline_counter
            .load(std::sync::atomic::Ordering::SeqCst);
        let current_gpu = engine.timeline_semaphore.get_timeline_value().unwrap_or(0);

        if current_gpu < target_timeline {
            engine
                .timeline_semaphore
                .wait_timeline(target_timeline, std::time::Duration::from_secs(5))
                .expect("[SliceMut Transfer] Timeline wait timed out before writing");
        }

        let element_size = self.stride as usize;
        let byte_offset = self.offset as u64 + (offset * element_size) as u64;
        let size_in_bytes = data.len() * element_size;

        let byte_data: &[u8] =
            unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, size_in_bytes) };

        engine
            .transfer_manager
            .write_buffer(&self._inner, byte_offset, byte_data)
            .expect("[SliceMut Transfer] Failed to write slice into GPU VRAM");
    }

    /// Reads all elements in this GPU slice back to a newly allocated host `Vec<T>`.
    ///
    /// # Diagnostics
    /// Halts execution with diagnostic `error[E2001]` if called inside an active `flow`.
    #[track_caller]
    pub fn to_vec(&self) -> Vec<T> {
        self.as_slice().to_vec()
    }

    /// Clones the contents of this slice into an independent, physical [`GpuVec<T>`] in VRAM.
    ///
    /// # Diagnostics
    /// Halts execution with diagnostic `error[E2001]` if called inside an active `flow`.
    pub fn to_gpu_vec(&self) -> GpuVec<T> {
        self.as_slice().to_gpu_vec()
    }
}
