use anu::nam_args_api::{ArgDescriptor, ArgValue, GpuType, IngressContext};
use std::marker::PhantomData;

/// A zero-cost hardware token used to allocate on-chip shared scratchpad memory for a spatial tile.
///
/// `GpuTileMem` does not allocate physical VRAM and has a zero-byte footprint in the `ParamArena`.
/// On GPU hardware, it maps directly to high-speed on-chip SRAM (`Workgroup` storage class / LDS).
///
/// # Nam Dispatch Semantics
/// Passing `GpuTileMem<T, N>` or `&GpuTileMem<T, N>` to a `#[nam]` function binds as an exclusive,
/// shared array reference **`&mut [T; N]`** across all threads within the same spatial tile.
///
/// # Synchronization & Memory Barrier
/// Threads within the tile coordinate access to this shared scratchpad via [`Space::sync()`].
///
/// # Diagnostics
/// Halts execution with diagnostic **`error[E1006]`** if total byte size (`N * size_of::<T>()`)
/// exceeds the physical GPU's maximum compute workgroup shared memory capacity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuTileMem<T, const N: usize> {
    _phantom: PhantomData<T>,
}

impl<T, const N: usize> GpuTileMem<T, N> {
    /// Creates a new `GpuTileMem` token representing an on-chip scratchpad of `N` elements of type `T`.
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }

    /// Returns the compile-time element capacity `N` of this tile scratchpad.
    #[inline(always)]
    pub const fn len(&self) -> usize {
        N
    }

    /// Returns `true` if the scratchpad element capacity is zero.
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        N == 0
    }
}

impl<T: Copy + Send + Sync + 'static, const N: usize> GpuType for GpuTileMem<T, N> {
    fn describe() -> ArgDescriptor {
        ArgDescriptor::workgroup_scratchpad::<T, N>()
    }

    fn collect<'a>(&self, ctx: &mut IngressContext<'a>) {
        let desc = Self::describe();
        ctx.push_arg(ArgValue::ZeroFootprint, desc);
    }
}

impl<T: Copy + Send + Sync + 'static, const N: usize> GpuType for &GpuTileMem<T, N> {
    fn describe() -> ArgDescriptor {
        ArgDescriptor::workgroup_scratchpad::<T, N>()
    }

    fn collect<'a>(&self, ctx: &mut IngressContext<'a>) {
        GpuType::collect(*self, ctx);
    }
}
