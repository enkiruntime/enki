use super::event::{Diagnostic, DynamicPayload};
use crate::diagnostics::source::Span;
use crate::nam_args_api::NamDispatchMap;
use crate::validation::borrow_engine::BorrowViolation;

pub struct ContractDiagnosticBuilder;

impl ContractDiagnosticBuilder {
    pub fn from_violation(violation: BorrowViolation, map: &NamDispatchMap) -> Diagnostic {
        let call_site = map.call_site;
        let contract = map.expected_contract;

        match violation {
            BorrowViolation::ArityMismatch {
                nam_name,
                nam_file,
                nam_line,
                expected_count,
                provided_count,
            } => {
                let mut diag = Diagnostic {
                    code: 1001,
                    spans: Vec::new(),
                    payload: Some(DynamicPayload::ArityMismatch {
                        nam_name,
                        expected_count,
                        provided_count,
                    }),
                };

                if let Some((file, line, col)) = call_site {
                    diag.add_span(Span::primary(file, line as usize, col as usize, 1));
                }
                diag.add_span(Span::secondary(nam_file, nam_line as usize, 1, 1));

                diag
            }

            BorrowViolation::MutabilityMismatch {
                param,
                provided_type_name,
                ..
            } => {
                let mut diag = Diagnostic {
                    code: 1002,
                    spans: Vec::new(),
                    payload: Some(DynamicPayload::MutabilityMismatch {
                        param_name: param.name,
                        expected_mut: param.is_mutable,
                        provided_type: provided_type_name,
                    }),
                };

                if let Some((file, line, col)) = call_site {
                    diag.add_span(Span::primary(file, line as usize, col as usize, 1));
                }
                if let Some(c) = contract {
                    diag.add_span(Span::secondary(
                        c.file_path,
                        param.line as usize,
                        param.column as usize,
                        param.name.len().max(1),
                    ));
                }

                diag
            }

            BorrowViolation::SemanticKindMismatch {
                param,
                expected_kind_str,
                provided_kind_str,
                ..
            } => {
                let mut diag = Diagnostic {
                    code: 1003,
                    spans: Vec::new(),
                    payload: Some(DynamicPayload::SemanticKindMismatch {
                        param_name: param.name,
                        expected_kind_str,
                        provided_kind_str,
                    }),
                };

                if let Some((file, line, col)) = call_site {
                    diag.add_span(Span::primary(file, line as usize, col as usize, 1));
                }
                if let Some(c) = contract {
                    diag.add_span(Span::secondary(
                        c.file_path,
                        param.line as usize,
                        param.column as usize,
                        param.name.len().max(1),
                    ));
                }

                diag
            }

            BorrowViolation::TypeMismatch {
                param,
                provided_type_name,
                ..
            } => {
                let mut diag = Diagnostic {
                    code: 1004,
                    spans: Vec::new(),
                    payload: Some(DynamicPayload::TypeMismatch {
                        param_name: param.name,
                        expected_type: param.type_str,
                        provided_type: provided_type_name,
                    }),
                };

                if let Some((file, line, col)) = call_site {
                    diag.add_span(Span::primary(file, line as usize, col as usize, 1));
                }
                if let Some(c) = contract {
                    diag.add_span(Span::secondary(
                        c.file_path,
                        param.line as usize,
                        param.column as usize,
                        param.name.len().max(1),
                    ));
                }

                diag
            }

            BorrowViolation::OverlappingSlices {
                type_name,
                first_arg_index,
                first_range,
                second_arg_index,
                second_range,
                intersection,
                ..
            } => {
                let mut diag = Diagnostic {
                    code: 1007,
                    spans: Vec::new(),
                    payload: Some(DynamicPayload::OverlappingSlices {
                        first_arg: first_arg_index,
                        first_range,
                        second_arg: second_arg_index,
                        second_range,
                        intersection,
                        element_type: type_name,
                    }),
                };

                if let Some((file, line, col)) = call_site {
                    diag.add_span(Span::primary(file, line as usize, col as usize, 1));
                }

                diag
            }

            BorrowViolation::SpaceDomainOverflow {
                arg_index,
                type_name,
                provided_elements,
                required_elements,
                space_dimensions,
            } => {
                let mut diag = Diagnostic {
                    code: 1008,
                    spans: Vec::new(),
                    payload: Some(DynamicPayload::SpaceDomainOverflow {
                        arg_index,
                        type_name,
                        provided_elements,
                        required_elements,
                        space_dimensions,
                    }),
                };

                if let Some((file, line, col)) = call_site {
                    diag.add_span(Span::primary(file, line as usize, col as usize, 1));
                }

                diag
            }

            BorrowViolation::UnrestrictedSliceMutInSafeMode {
                arg_index,
                type_name,
                element_range,
                total_cells,
            } => {
                let mut diag = Diagnostic {
                    code: 1009,
                    spans: Vec::new(),
                    payload: Some(DynamicPayload::UnrestrictedSliceMut {
                        arg_index,
                        type_name,
                        element_range,
                        total_cells,
                    }),
                };

                if let Some((file, line, col)) = call_site {
                    diag.add_span(Span::primary(file, line as usize, col as usize, 1));
                }

                diag
            }

            BorrowViolation::TemporalPresentationHazard { arg_index, root_id } => {
                let mut diag = Diagnostic {
                    code: 1010,
                    spans: Vec::new(),
                    payload: Some(DynamicPayload::TemporalPresentationHazard {
                        arg_index,
                        root_id,
                    }),
                };

                if let Some((file, line, col)) = call_site {
                    diag.add_span(Span::primary(file, line as usize, col as usize, 1));
                }

                diag
            }
        }
    }
}
