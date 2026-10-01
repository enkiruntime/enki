use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use anyhow::Context;
use apsu::{BufferUsage, GpuDeviceBuffer};

use anu::nam_args_api::{
    AccessIntent, ArgDescriptor, ArgValue, GpuBufferTarget, GpuType, IngressContext,
};

use crate::enki_api::context::active_engine;

/// Marks integer primitive types natively supported by GPU hardware atomic instructions.
///
/// Maps host scalar types (`u32`, `i32`, `u64`, etc.) to their standard library
/// atomic equivalents (`AtomicU32`, `AtomicI32`, etc.) inside `#[nam]` functions.
pub trait GpuAtomicTarget: Copy + Send + Sync + 'static {
    /// The corresponding standard library atomic type exposed inside GPU compute nam.
    type NamTarget: 'static;
}

impl GpuAtomicTarget for u32 {
    type NamTarget = core::sync::atomic::AtomicU32;
}

impl GpuAtomicTarget for i32 {
    type NamTarget = core::sync::atomic::AtomicI32;
}

impl GpuAtomicTarget for u64 {
    type NamTarget = core::sync::atomic::AtomicU64;
}

impl GpuAtomicTarget for i64 {
    type NamTarget = core::sync::atomic::AtomicI64;
}

impl GpuAtomicTarget for usize {
    type NamTarget = core::sync::atomic::AtomicUsize;
}

impl GpuAtomicTarget for isize {
    type NamTarget = core::sync::atomic::AtomicIsize;
}

/// A single, isolated hardware atomic variable residing in GPU VRAM.
///
/// Allocated as a GPU device storage buffer with direct 64-bit Buffer Device Address (BDA) support.
///
/// # Nam Dispatch Semantics
/// Passing `&GpuAtomic<T>` or `&mut GpuAtomic<T>` to a `#[nam]` function binds as a shared reference
/// to the standard library atomic type **`&T::NamTarget`** (e.g., `&AtomicU32`), permitting
/// atomic operations such as `.fetch_add()`, `.load()`, and `.store()`.
#[derive(Clone)]
pub struct GpuAtomic<T: GpuAtomicTarget> {
    pub slot_index: u32,
    pub device_address: u64,
    pub state: Arc<AtomicU8>,
    pub _inner: Arc<GpuDeviceBuffer>,
    _phantom: PhantomData<T>,
}

impl<T: GpuAtomicTarget> GpuAtomic<T> {
    /// Allocates a single hardware atomic variable in VRAM initialized with the given value.
    pub fn new(initial_value: T) -> Self {
        let engine = active_engine();
        let allocator = engine.allocator.clone();
        let transfer_manager = &engine.transfer_manager;

        let usage = BufferUsage::STORAGE_BUFFER
            | BufferUsage::SHADER_DEVICE_ADDRESS
            | BufferUsage::TRANSFER_DST
            | BufferUsage::TRANSFER_SRC;

        let size_bytes = std::mem::size_of::<T>() as u64;

        let device_buffer = GpuDeviceBuffer::new(allocator, size_bytes, usage)
            .map_err(|e| anyhow::anyhow!("[GpuAtomic] Failed to allocate device buffer: {}", e))
            .unwrap();

        let slot_index = device_buffer.id;
        let device_address = device_buffer.device_address();
        let state = device_buffer.state.clone();

        let bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(&initial_value as *const T as *const u8, size_bytes as usize)
        };

        transfer_manager
            .write_buffer(&device_buffer, 0, bytes)
            .map_err(|e| anyhow::anyhow!("[GpuAtomic] Failed to upload initial value: {}", e))
            .unwrap();

        Self {
            slot_index,
            device_address,
            state,
            _inner: Arc::new(device_buffer),
            _phantom: PhantomData,
        }
    }

    /// Reads the current value of the atomic variable from VRAM back to the host CPU.
    ///
    /// Automatically waits for in-flight GPU execution on the timeline before reading.
    ///
    /// # Diagnostics
    /// Halts execution with diagnostic **`error[E2001]`** if called inside an active `flow`.
    pub fn get(&self) -> T {
        self.try_get()
            .expect("[GpuAtomic] Failed to read atomic value from GPU VRAM")
    }

    fn try_get(&self) -> anyhow::Result<T> {
        let engine = active_engine();
        let last_value = engine.timeline_counter.load(Ordering::SeqCst);

        if last_value > 0 {
            engine
                .timeline_semaphore
                .wait_timeline(last_value, std::time::Duration::from_secs(5))
                .context("[GpuAtomic] Timeline wait failed before reading")?;
        }

        let mut out_val = unsafe { std::mem::zeroed::<T>() };
        let byte_data: &mut [u8] = unsafe {
            std::slice::from_raw_parts_mut(
                &mut out_val as *mut T as *mut u8,
                std::mem::size_of::<T>(),
            )
        };

        engine
            .transfer_manager
            .read_buffer(&self._inner, 0, byte_data)
            .context("[GpuAtomic] Failed to read atomic value")?;

        Ok(out_val)
    }

    /// Overwrites the atomic variable in VRAM from the host CPU.
    ///
    /// # Diagnostics
    /// Halts execution with diagnostic **`error[E2001]`** if called inside an active `flow`.
    pub fn set(&self, value: T) {
        self.try_set(value)
            .expect("[GpuAtomic] Failed to write atomic value to GPU VRAM");
    }

    fn try_set(&self, value: T) -> anyhow::Result<()> {
        let engine = active_engine();
        let byte_data: &[u8] = unsafe {
            std::slice::from_raw_parts(&value as *const T as *const u8, std::mem::size_of::<T>())
        };

        engine
            .transfer_manager
            .write_buffer(&self._inner, 0, byte_data)
            .map_err(|e| anyhow::anyhow!("[GpuAtomic] set failed: {}", e))
    }
}

impl<T: GpuAtomicTarget> GpuType for &GpuAtomic<T> {
    fn describe() -> ArgDescriptor {
        ArgDescriptor::atomic_cell::<T>()
    }

    fn collect<'a>(&self, ctx: &mut IngressContext<'a>) {
        let desc = Self::describe();
        ctx.push_arg(ArgValue::BufferBDA(self.device_address), desc);

        ctx.push_resource_binding(
            format!("GpuAtomic_Slot_{}", self.slot_index),
            self.slot_index,
            self.state.clone(),
            AccessIntent::ReadWrite,
        );
    }
}

/// A contiguous array of hardware atomic variables residing in GPU VRAM.
///
/// Designed for parallel GPU coordination algorithms such as spatial binning,
/// global work distribution, histogram calculations, and concurrent bucket accumulation.
///
/// # Nam Dispatch Semantics
/// Passing `&GpuAtomicVec<T>` or `&mut GpuAtomicVec<T>` to a `#[nam]` function binds as a slice of
/// standard library atomics **`&[T::NamTarget]`** (e.g., `&[AtomicU32]`).
///
/// In safe dispatch mode, the runtime `BorrowEngine` permits concurrent multi-threaded writes
/// because hardware atomic operations are safe across parallel threads by definition.
#[derive(Clone)]
pub struct GpuAtomicVec<T: GpuAtomicTarget> {
    pub slot_index: u32,
    pub element_count: usize,
    pub stride: u32,
    pub device_address: u64,
    pub state: Arc<AtomicU8>,
    pub _inner: Arc<GpuDeviceBuffer>,
    _phantom: PhantomData<T>,
}

impl<T: GpuAtomicTarget> GpuAtomicVec<T> {
    /// Allocates an array of hardware atomic variables in VRAM initialized with a host slice.
    pub fn new(data: &[T]) -> Self {
        let engine = active_engine();
        let allocator = engine.allocator.clone();
        let transfer_manager = &engine.transfer_manager;

        let usage = BufferUsage::STORAGE_BUFFER
            | BufferUsage::SHADER_DEVICE_ADDRESS
            | BufferUsage::TRANSFER_DST
            | BufferUsage::TRANSFER_SRC;

        let element_count = data.len();
        let element_size = std::mem::size_of::<T>();
        let size_bytes = std::mem::size_of_val(data) as u64;

        let device_buffer = GpuDeviceBuffer::new(allocator, size_bytes, usage)
            .map_err(|e| anyhow::anyhow!("[GpuAtomicVec] Failed to allocate device buffer: {}", e))
            .unwrap();

        let slot_index = device_buffer.id;
        let device_address = device_buffer.device_address();
        let state = device_buffer.state.clone();

        let bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(data.as_ptr() as *const u8, std::mem::size_of_val(data))
        };

        transfer_manager
            .write_buffer(&device_buffer, 0, bytes)
            .map_err(|e| anyhow::anyhow!("[GpuAtomicVec] Failed to upload initial data: {}", e))
            .unwrap();

        Self {
            slot_index,
            element_count,
            stride: element_size as u32,
            device_address,
            state,
            _inner: Arc::new(device_buffer),
            _phantom: PhantomData,
        }
    }

    /// Allocates an uninitialized array of hardware atomic variables with the specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let engine = active_engine();
        let allocator = engine.allocator.clone();

        let usage = BufferUsage::STORAGE_BUFFER
            | BufferUsage::SHADER_DEVICE_ADDRESS
            | BufferUsage::TRANSFER_DST
            | BufferUsage::TRANSFER_SRC;

        let element_size = std::mem::size_of::<T>();
        let size_bytes = (capacity * element_size) as u64;

        let device_buffer = GpuDeviceBuffer::new(allocator, size_bytes, usage)
            .map_err(|e| anyhow::anyhow!("[GpuAtomicVec] Allocation failed: {}", e))
            .unwrap();

        let slot_index = device_buffer.id;
        let device_address = device_buffer.device_address();
        let state = device_buffer.state.clone();

        Self {
            slot_index,
            element_count: capacity,
            stride: element_size as u32,
            device_address,
            state,
            _inner: Arc::new(device_buffer),
            _phantom: PhantomData,
        }
    }

    /// Returns the number of atomic variables in the buffer.
    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.element_count
    }

    /// Returns `true` if the buffer contains zero elements.
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.element_count == 0
    }

    /// Reads all atomic elements from VRAM back to a host `Vec<T>`.
    ///
    /// Automatically waits for in-flight GPU execution on the timeline before reading.
    ///
    /// # Diagnostics
    /// Halts execution with diagnostic **`error[E2001]`** if called inside an active `flow`.
    pub fn to_vec(&self) -> anyhow::Result<Vec<T>> {
        let engine = active_engine();
        let last_value = engine.timeline_counter.load(Ordering::SeqCst);

        if last_value > 0 {
            engine
                .timeline_semaphore
                .wait_timeline(last_value, std::time::Duration::from_secs(5))
                .context("[GpuAtomicVec] Timeline wait failed before reading")?;
        }

        let mut out_data = vec![unsafe { std::mem::zeroed::<T>() }; self.element_count];
        let byte_data: &mut [u8] = unsafe {
            std::slice::from_raw_parts_mut(
                out_data.as_mut_ptr() as *mut u8,
                out_data.len() * std::mem::size_of::<T>(),
            )
        };

        engine
            .transfer_manager
            .read_buffer(&self._inner, 0, byte_data)
            .context("[GpuAtomicVec] Failed to read atomic buffer data")?;

        Ok(out_data)
    }
}

impl<T: GpuAtomicTarget> GpuType for &GpuAtomicVec<T> {
    fn describe() -> ArgDescriptor {
        ArgDescriptor::atomic_slice::<T>()
    }

    fn collect<'a>(&self, ctx: &mut IngressContext<'a>) {
        let desc = Self::describe();
        ctx.push_arg(
            ArgValue::SliceBDA {
                bda: self.device_address,
                count: self.element_count as u64,
            },
            desc,
        );

        ctx.push_resource_binding(
            format!("GpuAtomicVec_Slot_{}", self.slot_index),
            self.slot_index,
            self.state.clone(),
            AccessIntent::ReadWrite,
        );
    }
}

impl<T: GpuAtomicTarget> GpuType for &mut GpuAtomic<T> {
    fn describe() -> ArgDescriptor {
        ArgDescriptor::atomic_cell::<T>()
    }

    fn collect<'a>(&self, ctx: &mut IngressContext<'a>) {
        let desc = Self::describe();
        ctx.push_arg(ArgValue::BufferBDA(self.device_address), desc);
        ctx.push_resource_binding(
            format!("GpuAtomic_Slot_{}", self.slot_index),
            self.slot_index,
            self.state.clone(),
            AccessIntent::ReadWrite,
        );
    }
}

impl<T: GpuAtomicTarget> GpuType for &mut GpuAtomicVec<T> {
    fn describe() -> ArgDescriptor {
        ArgDescriptor::atomic_slice::<T>()
    }

    fn collect<'a>(&self, ctx: &mut IngressContext<'a>) {
        let desc = Self::describe();
        ctx.push_arg(
            ArgValue::SliceBDA {
                bda: self.device_address,
                count: self.element_count as u64,
            },
            desc,
        );
        ctx.push_resource_binding(
            format!("GpuAtomicVec_Slot_{}", self.slot_index),
            self.slot_index,
            self.state.clone(),
            AccessIntent::ReadWrite,
        );
    }
}

impl<T: GpuAtomicTarget> GpuBufferTarget<T> for GpuAtomic<T> {}
impl<T: GpuAtomicTarget> GpuBufferTarget<T> for GpuAtomicVec<T> {}
