use super::core::GpuVec;
use crate::enki_api::context::active_engine;
use crate::enki_api::context::ambient::{is_inside_active_flow, resolve_user_caller};
use crate::enki_api::context::errors::emit_and_abort;

/// The temporal execution phase of a GPU buffer relative to the command timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferPhase {
    /// Buffer is idle and safe for immediate host manipulation.
    Idle,
    /// Buffer is currently involved in recording commands within an active flow.
    Recording,
    /// Buffer has commands submitted and executing on the GPU hardware.
    InFlight { target_timeline: u64 },
}

#[derive(Debug)]
pub struct PhaseViolationError {
    pub root_id: usize,
    pub phase: BufferPhase,
    pub operation: &'static str,
}

impl std::fmt::Display for PhaseViolationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "illegal host {op} on GpuVec (root id: {root}) during GPU command recording phase",
            op = self.operation,
            root = self.root_id,
        )
    }
}

impl std::error::Error for PhaseViolationError {}

impl<T: Copy + Send + Sync + 'static> GpuVec<T> {
    /// Queries the real-time execution phase of this buffer.
    pub fn current_phase(&self) -> BufferPhase {
        if is_inside_active_flow() {
            return BufferPhase::Recording;
        }

        let engine = active_engine();
        let target_timeline = engine
            .timeline_counter
            .load(std::sync::atomic::Ordering::SeqCst);
        let current_gpu_timeline = engine.timeline_semaphore.get_timeline_value().unwrap_or(0);

        if current_gpu_timeline < target_timeline {
            BufferPhase::InFlight { target_timeline }
        } else {
            BufferPhase::Idle
        }
    }

    #[track_caller]
    #[inline(always)]
    pub(crate) fn assert_host_readable(&self, operation: &'static str) {
        if let BufferPhase::Recording = self.current_phase() {
            let raw_caller = std::panic::Location::caller();
            let caller = resolve_user_caller(raw_caller);
            let diag = anu::diagnostics::rt::phase_violation(operation, caller);
            emit_and_abort(&diag);
        }
    }

    /// Returns `true` if the buffer is idle and safe for host access without waiting.
    #[inline(always)]
    pub fn is_idle(&self) -> bool {
        matches!(self.current_phase(), BufferPhase::Idle)
    }

    /// Returns `true` if the buffer is currently involved in an active flow recording.
    #[inline(always)]
    pub fn is_recording(&self) -> bool {
        matches!(self.current_phase(), BufferPhase::Recording)
    }
}
