use crate::diagnostics::source::Span;
use std::ops::Range;
use std::panic::Location;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DynamicPayload {
    VulkanDriverNotFound {
        error_details: String,
    },
    MissingHardwareFeatures {
        device_name: String,
        missing_features: Vec<&'static str>,
    },
    IntegratedGpuAlert {
        selected_gpu: String,
        available_dgpu: Option<String>,
    },
    ParamArenaClamped {
        requested_bytes: u64,
        max_supported_bytes: u64,
    },

    StackOverflow {
        required_bytes: u64,
        available_bytes: u64,
        total_bytes: u64,
        stack_per_cell: u32,
        total_cells: u64,
    },
    DimensionMismatch {
        expected_width: u32,
        expected_height: u32,
        actual_elements: usize,
    },
    PhaseViolation {
        operation: &'static str,
    },

    ArityMismatch {
        nam_name: &'static str,
        expected_count: usize,
        provided_count: usize,
    },
    MutabilityMismatch {
        param_name: &'static str,
        expected_mut: bool,
        provided_type: &'static str,
    },
    SemanticKindMismatch {
        param_name: &'static str,
        expected_kind_str: &'static str,
        provided_kind_str: &'static str,
    },
    TypeMismatch {
        param_name: &'static str,
        expected_type: &'static str,
        provided_type: &'static str,
    },

    OverlappingSlices {
        first_arg: usize,
        first_range: Range<usize>,
        second_arg: usize,
        second_range: Range<usize>,
        intersection: Range<usize>,
        element_type: &'static str,
    },
    SpaceDomainOverflow {
        arg_index: usize,
        type_name: &'static str,
        provided_elements: usize,
        required_elements: usize,
        space_dimensions: (usize, usize, usize),
    },
    UnrestrictedSliceMut {
        arg_index: usize,
        type_name: &'static str,
        element_range: Range<usize>,
        total_cells: usize,
    },
    TemporalPresentationHazard {
        arg_index: usize,
        root_id: usize,
    },

    TimestampQueriesExceeded {
        requested: u32,
        max_capacity: u32,
    },

    InternalCompilerError {
        token: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: u32,
    pub spans: Vec<Span>,
    pub payload: Option<DynamicPayload>,
}

impl Diagnostic {
    pub fn code_only(code: u32) -> Self {
        Self {
            code,
            spans: Vec::new(),
            payload: None,
        }
    }

    pub fn simple(code: u32, span: Span) -> Self {
        Self {
            code,
            spans: vec![span],
            payload: None,
        }
    }

    pub fn from_spans(code: u32, spans: Vec<Span>) -> Self {
        Self {
            code,
            spans,
            payload: None,
        }
    }

    pub fn from_caller(code: u32, caller: &'static Location<'static>) -> Self {
        let span = Span::primary(
            caller.file(),
            caller.line() as usize,
            caller.column() as usize,
            1,
        );
        Self::simple(code, span)
    }

    pub fn with_payload(mut self, payload: DynamicPayload) -> Self {
        self.payload = Some(payload);
        self
    }

    pub fn add_span(&mut self, span: Span) {
        self.spans.push(span);
    }

    pub fn primary_span(&self) -> Option<&Span> {
        self.spans.iter().find(|s| s.is_primary())
    }

    pub fn secondary_spans(&self) -> impl Iterator<Item = &Span> {
        self.spans.iter().filter(|s| s.is_secondary())
    }
}
