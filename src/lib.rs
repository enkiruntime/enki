pub mod enki_api;

// Core Execution Context
pub use enki_api::context::{
    Enki, EnkiBuilder, Flow, GpuInstant, active_engine, active_enki, set_active_enki,
};

// Spatial Domain & Topology
pub use enki_api::space::{IntoTile, Space, TileConfig};

// GPU Resources & Memory Types
pub use enki_api::resources::{
    GpuAtomic, GpuAtomicTarget, GpuAtomicVec, GpuParam, GpuTileMem, GpuVec, Slice, SliceMut,
};

// Safe Nam Execution Traits & Ingress Reflection
pub use enki_api::type_safety::*;

// Re-export contract types directly at root for macros
pub use anu::nam_args_api::{ExpectedParamKind, NamParamMeta, NamSignatureContract};

// Procedural Macro for GPU Nam Functions
pub use enki_macros::nam;

/// Convenient re-exports of common Enki types and traits.
pub mod prelude {
    pub use super::enki_api::context::{
        Enki, EnkiBuilder, Flow, GpuInstant, active_engine, active_enki, set_active_enki,
    };
    pub use super::enki_api::resources::{
        GpuAtomic, GpuAtomicTarget, GpuAtomicVec, GpuParam, GpuTileMem, GpuVec, Slice, SliceMut,
    };
    pub use super::enki_api::space::{IntoTile, Space, TileConfig};
    pub use super::enki_api::type_safety::*;
    pub use super::gpu_vec;
    pub use super::nam;
    pub use super::{ExpectedParamKind, NamParamMeta, NamSignatureContract};
}
