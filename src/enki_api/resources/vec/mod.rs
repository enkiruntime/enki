pub mod core;
pub mod dispatch;
pub mod format;
pub mod lifecycle;
pub mod ops;
pub mod state;
pub mod transfer;

pub use self::core::GpuVec;
pub use self::state::{BufferPhase, PhaseViolationError};

impl<T: Copy + Send + Sync + 'static> From<GpuVec<T>> for Vec<T> {
    #[inline(always)]
    fn from(gpu_vec: GpuVec<T>) -> Self {
        gpu_vec.to_vec()
    }
}

impl<T: Copy + Send + Sync + 'static> From<&[T]> for GpuVec<T> {
    #[inline(always)]
    fn from(slice: &[T]) -> Self {
        GpuVec::from_slice(slice)
    }
}

impl<T: Copy + Send + Sync + 'static> From<Vec<T>> for GpuVec<T> {
    #[inline(always)]
    fn from(vec: Vec<T>) -> Self {
        GpuVec::from_slice(&vec)
    }
}

/// Creates a [`GpuVec`] containing the provided arguments.
///
/// Matches standard Rust `vec!` syntax:
/// ```rust
/// use enki::gpu_vec;
///
/// // Explicit elements
/// let v1 = gpu_vec![1u32, 2, 3, 4];
///
/// // Repeating elements
/// let v2 = gpu_vec![0.0f32; 1024];
/// ```
#[macro_export]
macro_rules! gpu_vec {
    () => {
        $crate::enki_api::resources::GpuVec::new()
    };
    ($elem:expr; $n:expr) => {
        $crate::enki_api::resources::GpuVec::from_elem($elem, $n)
    };
    ($($x:expr),* $(,)?) => {
        $crate::enki_api::resources::GpuVec::from_slice(&[$($x),*])
    };
}
