pub mod context;
pub mod diagnostics;
pub mod nam_args_api;
pub mod pipeline_synthesis;
pub mod recording;
pub mod validation;

pub use context::{EngineConfig, EnkiEngine, EnkiEngineBuilder, TimelineCommandRing};

pub use nam_args_api::{
    AccessIntent, AccessMode, ArgDescriptor, ArgValue, BoundResource, DispatchSafetyMode,
    ExpectedParamKind, GpuBufferTarget, GpuType, IngressContext, InputResourceRecord, MemoryDomain,
    NamArgs, NamDispatchMap, NamParamMeta, NamSignatureContract, ProvidedArg, ResourceAccessKind,
    SpaceContract,
};

pub use recording::{
    BakeCommand, CompiledExecutionRecipe, ComputeTask, FrameCompiler, RawTask, TaskQueue,
};

pub use diagnostics::{
    ContractDiagnosticBuilder, Diagnostic, DynamicPayload, SourceCache, emit_batch, emit_diagnostic,
};

pub use validation::{
    BorrowEngine, BorrowViolation, FrameBorrowLedger, TaskBarriers, solve_optimal_barriers,
};

pub use pipeline_synthesis::{ComputeSynthesisInput, PipelineSynthesizer, TaskCompilationArtifact};

pub const ANU_VERSION: &str = env!("CARGO_PKG_VERSION");
