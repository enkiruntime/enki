use super::violation::BorrowViolation;
use crate::nam_args_api::{ExpectedParamKind, NamDispatchMap, ResourceAccessKind};

pub struct ContractMatcher;

impl ContractMatcher {
    pub fn verify(map: &NamDispatchMap) -> Result<(), BorrowViolation> {
        let contract = match map.expected_contract {
            Some(c) => c,
            None => return Ok(()),
        };

        let expected_count = contract.params.len();
        let provided_count = map.inputs.len();

        if expected_count != provided_count {
            return Err(BorrowViolation::ArityMismatch {
                nam_name: contract.nam_name,
                nam_file: contract.file_path,
                nam_line: contract.line,
                expected_count,
                provided_count,
            });
        }

        for (i, param) in contract.params.iter().enumerate() {
            let input = &map.inputs[i];

            if param.is_mutable && !input.is_mutable {
                return Err(BorrowViolation::MutabilityMismatch {
                    param: *param,
                    arg_index: input.arg_index,
                    provided_type_name: input.element_type_name,
                });
            }

            match param.expected_kind {
                ExpectedParamKind::PerCell => {
                    if !matches!(input.access_kind, ResourceAccessKind::PerCell { .. }) {
                        return Err(BorrowViolation::SemanticKindMismatch {
                            param: *param,
                            arg_index: input.arg_index,
                            expected_kind_str: "per-cell scalar reference (&T or &mut T)",
                            provided_kind_str: "indexed slice (Slice or SliceMut)",
                        });
                    }
                }
                ExpectedParamKind::Slice => {
                    if !matches!(input.access_kind, ResourceAccessKind::Slice { .. }) {
                        return Err(BorrowViolation::SemanticKindMismatch {
                            param: *param,
                            arg_index: input.arg_index,
                            expected_kind_str: "full indexed slice (&[T] or &mut [T])",
                            provided_kind_str: "per-cell vector (GpuVec)",
                        });
                    }
                }
                ExpectedParamKind::Atomic => {
                    if !matches!(input.access_kind, ResourceAccessKind::Atomic { .. }) {
                        return Err(BorrowViolation::SemanticKindMismatch {
                            param: *param,
                            arg_index: input.arg_index,
                            expected_kind_str: "hardware atomic reference (&AtomicT)",
                            provided_kind_str: "non-atomic buffer",
                        });
                    }
                }
                ExpectedParamKind::TileScratchpad => {
                    if !matches!(input.access_kind, ResourceAccessKind::TileScratchpad { .. }) {
                        return Err(BorrowViolation::SemanticKindMismatch {
                            param: *param,
                            arg_index: input.arg_index,
                            expected_kind_str: "tile shared memory (GpuTileMem)",
                            provided_kind_str: "global memory buffer",
                        });
                    }
                }
                ExpectedParamKind::ByValue => {
                    if !matches!(input.access_kind, ResourceAccessKind::ValueUniform { .. }) {
                        return Err(BorrowViolation::SemanticKindMismatch {
                            param: *param,
                            arg_index: input.arg_index,
                            expected_kind_str: "by-value uniform parameter (GpuParam)",
                            provided_kind_str: "buffer reference",
                        });
                    }
                }
            }

            if !param.type_str.contains(input.element_type_name) {
                return Err(BorrowViolation::TypeMismatch {
                    param: *param,
                    arg_index: input.arg_index,
                    provided_type_name: input.element_type_name,
                });
            }
        }

        Ok(())
    }
}
