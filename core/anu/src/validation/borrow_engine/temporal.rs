use super::violation::BorrowViolation;
use crate::nam_args_api::NamDispatchMap;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceFrameState {
    pub is_queued_for_presentation: bool,
    pub last_access_was_mut: bool,
}

#[derive(Debug, Default)]
pub struct FrameBorrowLedger {
    resources: HashMap<usize, ResourceFrameState>,
}

impl FrameBorrowLedger {
    pub fn new() -> Self {
        Self {
            resources: HashMap::with_capacity(32),
        }
    }

    pub fn record_dispatch(&mut self, map: &NamDispatchMap) -> Result<(), BorrowViolation> {
        for input in &map.inputs {
            let root_id = match input.root_id {
                Some(id) => id,
                None => continue,
            };

            if let Some(state) = self.resources.get(&root_id) {
                if state.is_queued_for_presentation && input.is_mutable {
                    return Err(BorrowViolation::TemporalPresentationHazard {
                        arg_index: input.arg_index,
                        root_id,
                    });
                }
            }

            let state = self.resources.entry(root_id).or_insert(ResourceFrameState {
                is_queued_for_presentation: false,
                last_access_was_mut: false,
            });

            state.last_access_was_mut = input.is_mutable;
        }

        Ok(())
    }

    pub fn mark_queued_for_presentation(&mut self, root_id: usize) {
        let state = self.resources.entry(root_id).or_insert(ResourceFrameState {
            is_queued_for_presentation: true,
            last_access_was_mut: false,
        });

        state.is_queued_for_presentation = true;
    }

    pub fn clear(&mut self) {
        self.resources.clear();
    }
}
