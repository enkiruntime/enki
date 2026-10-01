use std::ops::Sub;
use std::time::Duration;

use crate::enki_api::context::active_engine;
use crate::enki_api::context::ambient::{is_inside_active_flow, resolve_user_caller};
use crate::enki_api::context::errors::emit_and_abort;

/// A hardware-accurate timestamp query recorded directly on the GPU timeline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuInstant {
    pub(crate) query_slot: u32,
    pub(crate) timeline_value: u64,
    pub(crate) timestamp_period: f32,
}

impl GpuInstant {
    #[track_caller]
    fn assert_cpu_readable(&self, operation: &'static str) {
        if is_inside_active_flow() {
            let raw_caller = std::panic::Location::caller();
            let caller = resolve_user_caller(raw_caller);
            let diag = anu::diagnostics::rt::phase_violation(operation, caller);
            emit_and_abort(&diag);
        }
    }

    /// Computes the elapsed duration between this timestamp and an earlier GPU instant.
    ///
    /// # Diagnostics
    /// Halts execution if called inside an active `flow` before the GPU has retired the query.
    #[track_caller]
    pub fn duration_since(&self, earlier: &GpuInstant) -> Duration {
        self.assert_cpu_readable("duration_since on `GpuInstant`");

        let engine = active_engine();

        let max_timeline = self.timeline_value.max(earlier.timeline_value);
        let current_gpu = engine.timeline_semaphore.get_timeline_value().unwrap_or(0);

        if current_gpu < max_timeline {
            if let Err(_) = engine
                .timeline_semaphore
                .wait_timeline(max_timeline, Duration::from_secs(5))
            {
                let raw_caller = std::panic::Location::caller();
                let caller = resolve_user_caller(raw_caller);
                let diag = anu::diagnostics::rt::timeline_timeout(Some(caller));
                emit_and_abort(&diag);
            }
        }

        let ticks_end = engine
            .get_timestamp_query_result(self.query_slot)
            .expect("[GpuInstant] Failed to read end timestamp query");

        let ticks_start = engine
            .get_timestamp_query_result(earlier.query_slot)
            .expect("[GpuInstant] Failed to read start timestamp query");

        let delta_ticks = ticks_end.saturating_sub(ticks_start);
        let nanos = (delta_ticks as f64 * self.timestamp_period as f64) as u64;

        Duration::from_nanos(nanos)
    }

    /// Computes the elapsed duration from this instant until a later GPU instant.
    #[inline(always)]
    #[track_caller]
    pub fn elapsed_until(&self, later: &GpuInstant) -> Duration {
        later.duration_since(self)
    }
}

impl Sub for GpuInstant {
    type Output = Duration;

    #[inline(always)]
    #[track_caller]
    fn sub(self, other: GpuInstant) -> Duration {
        self.duration_since(&other)
    }
}

impl Sub<&GpuInstant> for &GpuInstant {
    type Output = Duration;

    #[inline(always)]
    #[track_caller]
    fn sub(self, other: &GpuInstant) -> Duration {
        self.duration_since(other)
    }
}
