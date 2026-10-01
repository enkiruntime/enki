use super::core::GpuVec;
use std::fmt;

impl<T: Copy + Send + Sync + 'static> GpuVec<T> {
    fn format_sample<F>(&self, f: &mut fmt::Formatter<'_>, mut format_elem: F) -> fmt::Result
    where
        F: FnMut(&T, &mut fmt::Formatter<'_>) -> fmt::Result,
    {
        if self.len == 0 {
            return write!(f, "[]");
        }

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

impl<T: Copy + Send + Sync + fmt::Debug + 'static> fmt::Debug for GpuVec<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.assert_host_readable("Debug");

        if f.alternate() {
            struct DataPreview<'a, T: Copy + Send + Sync + fmt::Debug + 'static>(&'a GpuVec<T>);

            impl<'a, T: Copy + Send + Sync + fmt::Debug + 'static> fmt::Debug for DataPreview<'a, T> {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    self.0
                        .format_sample(f, |item, formatter| write!(formatter, "{:?}", item))
                }
            }

            f.debug_struct("GpuVec")
                .field("slot", &self.slot_index)
                .field("len", &self.len)
                .field("capacity", &self.capacity)
                .field(
                    "device_address",
                    &format_args!("0x{:X}", self.device_address),
                )
                .field("vram_size", &format_args!("{} B", self.size_in_bytes))
                .field("data", &DataPreview(self))
                .finish()
        } else {
            self.format_sample(f, |item, formatter| write!(formatter, "{:?}", item))
        }
    }
}
