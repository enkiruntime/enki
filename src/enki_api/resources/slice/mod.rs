pub mod dispatch;
pub mod format;
pub(crate) mod range;
pub mod ro;
pub mod rw;

pub use self::ro::Slice;
pub use self::rw::SliceMut;
