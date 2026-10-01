use super::event::{Diagnostic, DynamicPayload};
use std::panic::Location;

pub fn driver_not_found(error_details: impl Into<String>) -> Diagnostic {
    Diagnostic::code_only(3000).with_payload(DynamicPayload::VulkanDriverNotFound {
        error_details: error_details.into(),
    })
}

pub fn stack_overflow(
    err: &apsu::StackAllocationError,
    grid_size: (u32, u32, u32),
    stack_per_cell: u32,
    caller: Option<&'static Location<'static>>,
) -> Diagnostic {
    let total_cells = (grid_size.0 as u64) * (grid_size.1 as u64) * (grid_size.2 as u64);

    let diag = if let Some(loc) = caller {
        Diagnostic::from_caller(3001, loc)
    } else {
        Diagnostic::code_only(3001)
    };

    diag.with_payload(DynamicPayload::StackOverflow {
        required_bytes: err.required_bytes,
        available_bytes: err.available_bytes,
        total_bytes: err.total_bytes,
        stack_per_cell,
        total_cells,
    })
}

pub fn out_of_vram(caller: Option<&'static Location<'static>>) -> Diagnostic {
    if let Some(loc) = caller {
        Diagnostic::from_caller(3002, loc)
    } else {
        Diagnostic::code_only(3002)
    }
}

pub fn tile_invocations_exceeded(caller: Option<&'static Location<'static>>) -> Diagnostic {
    if let Some(loc) = caller {
        Diagnostic::from_caller(3003, loc)
    } else {
        Diagnostic::code_only(3003)
    }
}

pub fn device_lost() -> Diagnostic {
    Diagnostic::code_only(3004)
}

pub fn missing_features(
    device_name: impl Into<String>,
    missing_features: Vec<&'static str>,
) -> Diagnostic {
    Diagnostic::code_only(3005).with_payload(DynamicPayload::MissingHardwareFeatures {
        device_name: device_name.into(),
        missing_features,
    })
}

pub fn headless_display_mismatch() -> Diagnostic {
    Diagnostic::code_only(3006)
}

pub fn zero_gpus_found() -> Diagnostic {
    Diagnostic::code_only(3007)
}

pub fn integrated_gpu_alert(
    selected_gpu: impl Into<String>,
    available_dgpu: Option<String>,
) -> Diagnostic {
    Diagnostic::code_only(3008).with_payload(DynamicPayload::IntegratedGpuAlert {
        selected_gpu: selected_gpu.into(),
        available_dgpu,
    })
}

pub fn param_arena_clamped(requested_bytes: u64, max_supported_bytes: u64) -> Diagnostic {
    Diagnostic::code_only(3009).with_payload(DynamicPayload::ParamArenaClamped {
        requested_bytes,
        max_supported_bytes,
    })
}

pub fn param_arena_overflow(requested_bytes: u64, max_supported_bytes: u64) -> Diagnostic {
    Diagnostic::code_only(3010).with_payload(DynamicPayload::ParamArenaClamped {
        requested_bytes,
        max_supported_bytes,
    })
}

pub fn timestamp_queries_exceeded(
    requested: u32,
    max_capacity: u32,
    caller: Option<&'static Location<'static>>,
) -> Diagnostic {
    let diag = if let Some(loc) = caller {
        Diagnostic::from_caller(3011, loc)
    } else {
        Diagnostic::code_only(3011)
    };

    diag.with_payload(DynamicPayload::TimestampQueriesExceeded {
        requested,
        max_capacity,
    })
}
