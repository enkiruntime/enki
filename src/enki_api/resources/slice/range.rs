use std::ops::{Bound, RangeBounds};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedRange {
    pub start: usize,
    pub count: usize,
    pub end: usize,
}

impl ResolvedRange {
    #[track_caller]
    pub(crate) fn resolve<R: RangeBounds<usize>>(
        range: R,
        total_len: usize,
        caller: &'static std::panic::Location<'static>,
    ) -> Self {
        let start = match range.start_bound() {
            Bound::Included(&s) => s,
            Bound::Excluded(&s) => s
                .checked_add(1)
                .expect("[Slice Range] Start bound integer overflow"),
            Bound::Unbounded => 0,
        };

        let end = match range.end_bound() {
            Bound::Included(&e) => e
                .checked_add(1)
                .expect("[Slice Range] End bound integer overflow"),
            Bound::Excluded(&e) => e,
            Bound::Unbounded => total_len,
        };

        if start > end || end > total_len {
            let diag = anu::diagnostics::rt::slice_out_of_bounds(caller);
            crate::enki_api::context::errors::emit_and_abort(&diag);
        }

        let count = end - start;

        Self { start, count, end }
    }
}
