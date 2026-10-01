use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::AtomicU8;

use crate::enki_api::context::active_engine;
use apsu::{BufferUsage, GpuDeviceBuffer};

/// A contiguous, growable array allocated in GPU VRAM with exclusive ownership semantics.
///
/// `GpuVec<T>` mirrors standard Rust `Vec<T>` ergonomics while residing entirely
/// in GPU memory via 64-bit Buffer Device Addresses (BDA).
///
/// # Behavior & Invariants
/// - **Exclusive Ownership:** Follows Rust's move semantics. Calling `clone()` performs
///   a physical VRAM-to-VRAM deep copy with a distinct memory range.
/// - **SPMD Kernel Dispatch:** Passing `&GpuVec<T>` or `&mut GpuVec<T>` to a `#[nam]`
///   binds per-thread as `&T` or `&mut T` (1:1 cell access). The vector's `len()` must
///   be greater than or equal to the total execution domain (`space.size_x * space.size_y * space.size_z`),
///   otherwise dispatch halts with diagnostic **`error[E1008]`** (`SpaceDomainOverflow`).
/// - **Zero-Sized Types:** Zero-sized types (ZST) are strictly forbidden on GPU hardware.
pub struct GpuVec<T: Copy + Send + Sync + 'static> {
    pub(crate) slot_index: u32,
    pub(crate) offset: u32,
    pub(crate) stride: u32,
    pub(crate) len: usize,
    pub(crate) capacity: usize,
    pub(crate) device_address: u64,
    pub(crate) size_in_bytes: u64,
    pub(crate) state: Arc<AtomicU8>,
    pub(crate) _inner: Arc<GpuDeviceBuffer>,
    pub(crate) _phantom: PhantomData<T>,
}

impl<T: Copy + Send + Sync + 'static> GpuVec<T> {
    pub(crate) fn allocate_raw(capacity: usize) -> (Arc<GpuDeviceBuffer>, u64, u32, u64) {
        let element_size = std::mem::size_of::<T>();
        assert!(
            element_size > 0,
            "[GpuVec] Zero-Sized Types (ZST) are not supported on GPU hardware!"
        );

        let engine = active_engine();
        let allocator = engine.allocator.clone();

        let usage = BufferUsage::STORAGE_BUFFER
            | BufferUsage::SHADER_DEVICE_ADDRESS
            | BufferUsage::TRANSFER_DST
            | BufferUsage::TRANSFER_SRC;

        let alloc_capacity = capacity.max(1);
        let size_in_bytes = (alloc_capacity * element_size) as u64;

        let device_buffer = match GpuDeviceBuffer::new(allocator, size_in_bytes, usage) {
            Ok(buf) => buf,
            Err(_) => {
                let diag = anu::diagnostics::hw::out_of_vram(None);
                crate::enki_api::context::errors::emit_and_abort(&diag);
            }
        };

        let device_address = device_buffer.device_address();
        let slot_index = device_buffer.id;

        (
            Arc::new(device_buffer),
            device_address,
            slot_index,
            size_in_bytes,
        )
    }

    /// Constructs a new, empty `GpuVec<T>` without pre-allocated VRAM capacity.
    #[inline(always)]
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Creates an empty `GpuVec<T>` with pre-allocated VRAM capacity for `capacity` elements.
    pub fn with_capacity(capacity: usize) -> Self {
        let element_size = std::mem::size_of::<T>() as u32;
        let (buffer, device_address, slot_index, size_in_bytes) = Self::allocate_raw(capacity);

        Self {
            slot_index,
            offset: 0,
            stride: element_size,
            len: 0,
            capacity,
            device_address,
            size_in_bytes,
            state: buffer.state.clone(),
            _inner: buffer,
            _phantom: PhantomData,
        }
    }

    /// Creates a new `GpuVec<T>` initialized with data copied from a host CPU slice.
    pub fn from_slice(data: &[T]) -> Self {
        let count = data.len();
        let element_size = std::mem::size_of::<T>();
        assert!(
            element_size > 0,
            "[GpuVec] Zero-Sized Types (ZST) are not supported on GPU hardware."
        );

        let (buffer, device_address, slot_index, size_in_bytes) = Self::allocate_raw(count);

        if count > 0 {
            let engine = active_engine();
            let bytes: &[u8] = unsafe {
                std::slice::from_raw_parts(data.as_ptr() as *const u8, size_of_val(data))
            };

            engine
                .transfer_manager
                .write_buffer(&buffer, 0, bytes)
                .expect("[GpuVec] Failed to upload initial data to GPU VRAM");
        }

        Self {
            slot_index,
            offset: 0,
            stride: element_size as u32,
            len: count,
            capacity: count,
            device_address,
            size_in_bytes,
            state: buffer.state.clone(),
            _inner: buffer,
            _phantom: PhantomData,
        }
    }

    /// Creates a `GpuVec<T>` of size `count` with each element initialized to `value`.
    pub fn from_elem(value: T, count: usize) -> Self {
        let host_data = vec![value; count];
        Self::from_slice(&host_data)
    }

    /// Creates a `GpuVec<T>` of size `count` initialized with zeroed VRAM memory.
    pub fn zeroed(count: usize) -> Self {
        let element_size = std::mem::size_of::<T>();
        let (buffer, device_address, slot_index, size_in_bytes) = Self::allocate_raw(count);

        if count > 0 {
            let engine = active_engine();
            let zero_bytes = vec![0u8; count * element_size];
            engine
                .transfer_manager
                .write_buffer(&buffer, 0, &zero_bytes)
                .expect("[GpuVec] Failed to zero-initialize GPU VRAM");
        }

        Self {
            slot_index,
            offset: 0,
            stride: element_size as u32,
            len: count,
            capacity: count,
            device_address,
            size_in_bytes,
            state: buffer.state.clone(),
            _inner: buffer,
            _phantom: PhantomData,
        }
    }

    /// Returns the number of active elements in the vector.
    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns the maximum number of elements the vector can hold without reallocating VRAM.
    #[inline(always)]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns `true` if the vector contains no elements.
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the size in bytes of a single element `T`.
    #[inline(always)]
    pub const fn stride(&self) -> usize {
        self.stride as usize
    }

    /// Returns the 64-bit GPU Buffer Device Address (BDA) pointing directly to this VRAM allocation.
    #[inline(always)]
    pub const fn device_address(&self) -> u64 {
        self.device_address
    }

    /// Returns the internal engine slot index used for synchronization and resource tracking.
    #[inline(always)]
    pub const fn slot_index(&self) -> u32 {
        self.slot_index
    }

    /// Returns the total physical size allocated in VRAM in bytes.
    #[inline(always)]
    pub const fn size_in_bytes(&self) -> u64 {
        self.size_in_bytes
    }
}

impl<T: Copy + Send + Sync + 'static> Default for GpuVec<T> {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}
