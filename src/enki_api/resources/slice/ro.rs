use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ops::{Range, RangeBounds};
use std::sync::Arc;
use std::sync::atomic::AtomicU8;

use super::range::ResolvedRange;
use crate::enki_api::context::active_engine;
use crate::enki_api::context::ambient::{is_inside_active_flow, resolve_user_caller};
use crate::enki_api::context::errors::emit_and_abort;
use crate::enki_api::resources::GpuVec;
use apsu::GpuDeviceBuffer;

/// An immutable, zero-allocation borrowed view into a contiguous sub-range of GPU memory.
///
/// Unlike [`GpuVec`], a `Slice` does not own or allocate physical VRAM; it represents
/// a lightweight view window with a 64-bit Buffer Device Address (BDA) and element bounds.
///
/// # Nam Dispatch Semantics
/// Passing `&Slice<T>` to a `#[nam]` function binds as a global read-only slice **`&[T]`**,
/// granting parallel GPU threads indexed read access across the entire slice bounds.
///
/// Unlike mutable slices, multiple immutable `Slice` views from the same parent buffer
/// are explicitly permitted to overlap during kernel dispatches.
pub struct Slice<'a, T: Copy + Send + Sync + 'static> {
    pub(crate) slot_index: u32,
    pub(crate) offset: u32,
    pub(crate) element_offset: usize,
    pub(crate) stride: u32,
    pub(crate) len: usize,
    pub(crate) device_address: u64,
    pub(crate) state: Arc<AtomicU8>,
    pub(crate) _inner: Arc<GpuDeviceBuffer>,
    pub(crate) _phantom: PhantomData<&'a T>,
}

impl<'a, T: Copy + Send + Sync + 'static> Clone for Slice<'a, T> {
    fn clone(&self) -> Self {
        Self {
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
}

impl<'a, T: Copy + Send + Sync + 'static> Slice<'a, T> {
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

    /// Sub-slices this view into a smaller contiguous sub-range.
    ///
    /// # Diagnostics
    /// Halts execution with diagnostic `error[E2005]` if `range` exceeds the slice bounds.
    #[track_caller]
    pub fn slice<R: RangeBounds<usize>>(&self, range: R) -> Slice<'a, T> {
        let caller = std::panic::Location::caller();
        let resolved = ResolvedRange::resolve(range, self.len, caller);
        let byte_offset = (resolved.start * self.stride as usize) as u64;

        Slice {
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

    /// Splits the slice into two disjoint sub-slices at index `mid`.
    ///
    /// # Panics
    /// Panics if `mid > self.len()`.
    pub fn split_at(&self, mid: usize) -> (Slice<'a, T>, Slice<'a, T>) {
        (self.slice(..mid), self.slice(mid..))
    }

    #[track_caller]
    fn assert_host_readable(&self, operation: &'static str) {
        if is_inside_active_flow() {
            let raw_caller = std::panic::Location::caller();
            let caller = resolve_user_caller(raw_caller);
            let diag = anu::diagnostics::rt::phase_violation(operation, caller);
            emit_and_abort(&diag);
        }
    }

    /// Reads back a single element from this GPU slice to the host CPU at the specified index.
    ///
    /// Automatically synchronizes with the GPU timeline if the underlying buffer is currently in-flight.
    ///
    /// # Diagnostics
    /// Halts execution with diagnostic `error[E2001]` if called inside an active `flow`.
    #[track_caller]
    pub fn get(&self, index: usize) -> Option<T> {
        if index >= self.len {
            return None;
        }

        self.assert_host_readable("get on `Slice`");

        let engine = active_engine();
        let target_timeline = engine
            .timeline_counter
            .load(std::sync::atomic::Ordering::SeqCst);
        let current_gpu = engine.timeline_semaphore.get_timeline_value().unwrap_or(0);

        if current_gpu < target_timeline {
            engine
                .timeline_semaphore
                .wait_timeline(target_timeline, std::time::Duration::from_secs(5))
                .expect("[Slice Transfer] Timeline wait timed out before reading element");
        }

        let element_size = self.stride as usize;
        let byte_offset = self.offset as u64 + (index * element_size) as u64;

        let mut out_val = MaybeUninit::<T>::uninit();
        let byte_slice: &mut [u8] = unsafe {
            std::slice::from_raw_parts_mut(out_val.as_mut_ptr() as *mut u8, element_size)
        };

        engine
            .transfer_manager
            .read_buffer(&self._inner, byte_offset, byte_slice)
            .expect("[Slice Transfer] Failed to read element from GPU VRAM");

        unsafe { Some(out_val.assume_init()) }
    }

    /// Reads all elements in this GPU slice back to a newly allocated host `Vec<T>`.
    ///
    /// # Diagnostics
    /// Halts execution with diagnostic `error[E2001]` if called inside an active `flow`.
    #[track_caller]
    pub fn to_vec(&self) -> Vec<T> {
        if self.len == 0 {
            return Vec::new();
        }

        self.assert_host_readable("to_vec on `Slice`");

        let engine = active_engine();
        let target_timeline = engine
            .timeline_counter
            .load(std::sync::atomic::Ordering::SeqCst);
        let current_gpu = engine.timeline_semaphore.get_timeline_value().unwrap_or(0);

        if current_gpu < target_timeline {
            engine
                .timeline_semaphore
                .wait_timeline(target_timeline, std::time::Duration::from_secs(10))
                .expect("[Slice Transfer] Timeline wait timed out before reading slice");
        }

        let element_size = self.stride as usize;
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
            .expect("[Slice Transfer] Failed to read slice bytes from GPU VRAM");

        unsafe {
            let mut manual_vec = std::mem::ManuallyDrop::new(uninit_vec);
            Vec::from_raw_parts(
                manual_vec.as_mut_ptr() as *mut T,
                manual_vec.len(),
                manual_vec.capacity(),
            )
        }
    }

    /// Clones the contents of this slice into an independent, physical [`GpuVec<T>`] in VRAM.
    ///
    /// Matches `std::slice::to_vec` but allocates physical VRAM instead of host memory.
    ///
    /// # Diagnostics
    /// Halts execution with diagnostic `error[E2001]` if called inside an active `flow`.
    pub fn to_gpu_vec(&self) -> GpuVec<T> {
        let cpu_data = self.to_vec();
        GpuVec::from_slice(&cpu_data)
    }
}
