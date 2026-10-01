use super::ro::Slice;
use super::rw::SliceMut;
use crate::enki_api::context::ambient::{is_inside_active_flow, resolve_user_caller};
use crate::enki_api::context::errors::emit_and_abort;
use std::fmt;

impl<'a, T: Copy + Send + Sync + 'static> Slice<'a, T> {
    #[track_caller]
    fn assert_host_printable(&self, operation: &'static str) {
        if is_inside_active_flow() {
            let raw_caller = std::panic::Location::caller();
            let caller = resolve_user_caller(raw_caller);
            let diag = anu::diagnostics::rt::phase_violation(operation, caller);
            emit_and_abort(&diag);
        }
    }

    #[track_caller]
    fn format_sample<F>(&self, f: &mut fmt::Formatter<'_>, mut format_elem: F) -> fmt::Result
    where
        F: FnMut(&T, &mut fmt::Formatter<'_>) -> fmt::Result,
    {
        if self.len == 0 {
            return write!(f, "[]");
        }

        self.assert_host_printable("printing of `Slice`");

        const MAX_FULL_DISPLAY: usize = 10_000;
        const EDGE_SAMPLE_COUNT: usize = 64;

        if self.len <= MAX_FULL_DISPLAY {
            let data = self.to_vec();
            write!(f, "[")?;
            for (i, item) in data.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                format_elem(item, f)?;
            }
            write!(f, "]")
        } else {
            write!(f, "[")?;
            for i in 0..EDGE_SAMPLE_COUNT {
                if let Some(item) = self.get(i) {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    format_elem(&item, f)?;
                }
            }

            let omitted = self.len - (EDGE_SAMPLE_COUNT * 2);
            write!(f, ", ... ({} elements omitted) ..., ", omitted)?;

            for i in (self.len - EDGE_SAMPLE_COUNT)..self.len {
                if let Some(item) = self.get(i) {
                    if i > (self.len - EDGE_SAMPLE_COUNT) {
                        write!(f, ", ")?;
                    }
                    format_elem(&item, f)?;
                }
            }
            write!(f, "]")
        }
    }
}

impl<'a, T: Copy + Send + Sync + fmt::Debug + 'static> fmt::Debug for Slice<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.assert_host_printable("debug formatting of `Slice`");

        if f.alternate() {
            struct DataPreview<'a, 'b, T: Copy + Send + Sync + fmt::Debug + 'static>(
                &'b Slice<'a, T>,
            );

            impl<'a, 'b, T: Copy + Send + Sync + fmt::Debug + 'static> fmt::Debug for DataPreview<'a, 'b, T> {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    self.0
                        .format_sample(f, |item, formatter| write!(formatter, "{:?}", item))
                }
            }

            f.debug_struct("Slice")
                .field("root_id", &self.root_id())
                .field("element_range", &self.element_range())
                .field("len", &self.len)
                .field("stride", &self.stride)
                .field("data", &DataPreview(self))
                .finish()
        } else {
            self.format_sample(f, |item, formatter| write!(formatter, "{:?}", item))
        }
    }
}

impl<'a, T: Copy + Send + Sync + fmt::Debug + 'static> fmt::Debug for SliceMut<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_slice()
            .assert_host_printable("debug formatting of `SliceMut`");

        if f.alternate() {
            struct DataPreview<'a, 'b, T: Copy + Send + Sync + fmt::Debug + 'static>(
                &'b SliceMut<'a, T>,
            );

            impl<'a, 'b, T: Copy + Send + Sync + fmt::Debug + 'static> fmt::Debug for DataPreview<'a, 'b, T> {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    self.0
                        .as_slice()
                        .format_sample(f, |item, formatter| write!(formatter, "{:?}", item))
                }
            }

            f.debug_struct("SliceMut")
                .field("root_id", &self.root_id())
                .field("element_range", &self.element_range())
                .field("len", &self.len)
                .field("stride", &self.stride)
                .field("data", &DataPreview(self))
                .finish()
        } else {
            self.as_slice()
                .format_sample(f, |item, formatter| write!(formatter, "{:?}", item))
        }
    }
}
