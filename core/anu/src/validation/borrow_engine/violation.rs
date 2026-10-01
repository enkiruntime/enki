use crate::nam_args_api::NamParamMeta;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BorrowViolation {
    ArityMismatch {
        nam_name: &'static str,
        nam_file: &'static str,
        nam_line: u32,
        expected_count: usize,
        provided_count: usize,
    },

    MutabilityMismatch {
        param: NamParamMeta,
        arg_index: usize,
        provided_type_name: &'static str,
    },

    SemanticKindMismatch {
        param: NamParamMeta,
        arg_index: usize,
        expected_kind_str: &'static str,
        provided_kind_str: &'static str,
    },

    TypeMismatch {
        param: NamParamMeta,
        arg_index: usize,
        provided_type_name: &'static str,
    },

    SpaceDomainOverflow {
        arg_index: usize,
        type_name: &'static str,
        provided_elements: usize,
        required_elements: usize,
        space_dimensions: (usize, usize, usize),
    },

    OverlappingSlices {
        root_id: usize,
        type_name: &'static str,
        first_arg_index: usize,
        first_range: Range<usize>,
        first_is_mut: bool,
        second_arg_index: usize,
        second_range: Range<usize>,
        second_is_mut: bool,
        intersection: Range<usize>,
    },

    UnrestrictedSliceMutInSafeMode {
        arg_index: usize,
        type_name: &'static str,
        element_range: Range<usize>,
        total_cells: usize,
    },

    TemporalPresentationHazard {
        arg_index: usize,
        root_id: usize,
    },
}
