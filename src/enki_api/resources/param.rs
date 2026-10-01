use anu::nam_args_api::{ArgDescriptor, ArgValue, GpuType, IngressContext};
use std::ops::{Deref, DerefMut};

/// Container for uniforms and small configuration structs passed directly by value into GPU nam functions.
///
/// Unlike buffer resources, `GpuParam<T>` does not allocate an independent VRAM buffer.
/// Its byte payload is packed directly into the engine's uniform `ParamArena` per dispatch.
///
/// # Nam Dispatch Semantics
/// Passing `GpuParam<T>` or `&GpuParam<T>` to a `#[nam]` function binds directly by-value
/// as **`T`** inside the nam function signature (e.g., `dt: f32`, `cam: CameraParams`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct GpuParam<T> {
    /// The inner uniform value.
    pub value: T,
}

impl<T> GpuParam<T> {
    /// Creates a new `GpuParam` wrapping the provided value.
    #[inline(always)]
    pub const fn new(value: T) -> Self {
        Self { value }
    }

    /// Updates the inner value.
    #[inline(always)]
    pub fn set(&mut self, value: T) {
        self.value = value;
    }

    /// Gets an immutable reference to the inner value.
    #[inline(always)]
    pub const fn get(&self) -> &T {
        &self.value
    }
}

impl<T> From<T> for GpuParam<T> {
    #[inline(always)]
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T> Deref for GpuParam<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for GpuParam<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T: Copy + Send + Sync + 'static> GpuType for GpuParam<T> {
    fn describe() -> ArgDescriptor {
        ArgDescriptor::by_value::<T>()
    }

    fn collect<'a>(&self, ctx: &mut IngressContext<'a>) {
        let size = std::mem::size_of::<T>();
        let bytes =
            unsafe { std::slice::from_raw_parts(&self.value as *const T as *const u8, size) }
                .to_vec();

        let desc = Self::describe();
        ctx.push_arg(ArgValue::Payload(bytes), desc);
    }
}

impl<T: Copy + Send + Sync + 'static> GpuType for &GpuParam<T> {
    fn describe() -> ArgDescriptor {
        ArgDescriptor::by_value::<T>()
    }

    fn collect<'a>(&self, ctx: &mut IngressContext<'a>) {
        GpuType::collect(*self, ctx);
    }
}
