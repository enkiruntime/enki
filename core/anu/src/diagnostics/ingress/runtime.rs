use super::event::{Diagnostic, DynamicPayload};
use std::panic::Location;

pub fn phase_violation(operation: &'static str, caller: &'static Location<'static>) -> Diagnostic {
    Diagnostic::from_caller(2001, caller).with_payload(DynamicPayload::PhaseViolation { operation })
}

pub fn present_dimension_mismatch(
    expected_width: u32,
    expected_height: u32,
    actual_elements: usize,
    caller: &'static Location<'static>,
) -> Diagnostic {
    Diagnostic::from_caller(2002, caller).with_payload(DynamicPayload::DimensionMismatch {
        expected_width,
        expected_height,
        actual_elements,
    })
}

pub fn outside_frame(caller: &'static Location<'static>) -> Diagnostic {
    Diagnostic::from_caller(2003, caller)
}

pub fn nested_frame(caller: &'static Location<'static>) -> Diagnostic {
    Diagnostic::from_caller(2004, caller)
}

pub fn slice_out_of_bounds(caller: &'static Location<'static>) -> Diagnostic {
    Diagnostic::from_caller(2005, caller)
}

pub fn timeline_timeout(caller: Option<&'static Location<'static>>) -> Diagnostic {
    if let Some(loc) = caller {
        Diagnostic::from_caller(2006, loc)
    } else {
        Diagnostic {
            code: 2006,
            spans: Vec::new(),
            payload: None,
        }
    }
}
