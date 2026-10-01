use super::violation::BorrowViolation;
use crate::nam_args_api::{NamDispatchMap, ResourceAccessKind};

pub struct SpatialBorrowChecker;

impl SpatialBorrowChecker {
    pub fn verify(map: &NamDispatchMap) -> Result<(), BorrowViolation> {
        Self::check_space_domain_bounds(map)?;
        Self::check_slice_disjointness(map)?;
        Self::check_slice_mut_safety_policy(map)?;

        Ok(())
    }

    fn check_space_domain_bounds(map: &NamDispatchMap) -> Result<(), BorrowViolation> {
        let required_cells = map.space.total_cells;

        for input in &map.inputs {
            if let ResourceAccessKind::PerCell { element_count } = input.access_kind {
                if element_count < required_cells {
                    return Err(BorrowViolation::SpaceDomainOverflow {
                        arg_index: input.arg_index,
                        type_name: input.element_type_name,
                        provided_elements: element_count,
                        required_elements: required_cells,
                        space_dimensions: map.space.dimensions,
                    });
                }
            }
        }

        Ok(())
    }

    fn check_slice_disjointness(map: &NamDispatchMap) -> Result<(), BorrowViolation> {
        let inputs_count = map.inputs.len();

        for i in 0..inputs_count {
            let a = &map.inputs[i];

            let a_root = match a.root_id {
                Some(id) => id,
                None => continue,
            };

            let a_range = match a.as_slice_range() {
                Some(r) => r.clone(),
                None => continue,
            };

            for j in (i + 1)..inputs_count {
                let b = &map.inputs[j];

                let b_root = match b.root_id {
                    Some(id) => id,
                    None => continue,
                };

                if a_root != b_root {
                    continue;
                }

                let b_range = match b.as_slice_range() {
                    Some(r) => r.clone(),
                    None => continue,
                };

                if !a.is_mutable && !b.is_mutable {
                    continue;
                }

                let overlap_start = a_range.start.max(b_range.start);
                let overlap_end = a_range.end.min(b_range.end);

                if overlap_start < overlap_end {
                    let intersection = overlap_start..overlap_end;

                    return Err(BorrowViolation::OverlappingSlices {
                        root_id: a_root,
                        type_name: a.element_type_name,
                        first_arg_index: a.arg_index,
                        first_range: a_range,
                        first_is_mut: a.is_mutable,
                        second_arg_index: b.arg_index,
                        second_range: b_range,
                        second_is_mut: b.is_mutable,
                        intersection,
                    });
                }
            }
        }

        Ok(())
    }

    fn check_slice_mut_safety_policy(map: &NamDispatchMap) -> Result<(), BorrowViolation> {
        if map.safety_mode.is_unchecked() {
            return Ok(());
        }

        if map.space.total_cells <= 1 {
            return Ok(());
        }

        for input in &map.inputs {
            if input.is_mutable {
                if let ResourceAccessKind::Slice { element_range, .. } = &input.access_kind {
                    return Err(BorrowViolation::UnrestrictedSliceMutInSafeMode {
                        arg_index: input.arg_index,
                        type_name: input.element_type_name,
                        element_range: element_range.clone(),
                        total_cells: map.space.total_cells,
                    });
                }
            }
        }

        Ok(())
    }
}
