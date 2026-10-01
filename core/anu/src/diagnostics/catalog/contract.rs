use super::entry::DiagnosticTemplate;

pub fn lookup(code: u32) -> Option<DiagnosticTemplate> {
    match code {
        1001 => Some(DiagnosticTemplate::error(
            1001,
            "incorrect number of arguments supplied in nam dispatch",
            "argument count mismatch occurs here",
            Some("nam function defined with different parameter count here"),
            Some(
                "each parameter declared in the `#[nam]` function signature must be provided sequentially in the dispatch call.",
            ),
            Some("verify that all declared parameters are passed, or remove extraneous arguments."),
        )),

        1002 => Some(DiagnosticTemplate::error(
            1002,
            "mismatched reference mutability in nam dispatch",
            "passed as an immutable shared reference `&`",
            Some("parameter declared as an exclusive mutable reference `&mut` here"),
            Some(
                "the nam parameter requires an exclusive mutable borrow to write data, but the caller supplied a shared read-only reference.",
            ),
            Some(
                "ensure the container is declared as mutable (`let mut ...`) and passed using an exclusive borrow (`&mut`).",
            ),
        )),

        1003 => Some(DiagnosticTemplate::error(
            1003,
            "mismatched container access kind in nam dispatch",
            "passed a slice, but the nam expects a per-cell scalar reference",
            Some("parameter declared as a single-element scalar reference here"),
            Some(
                "passing `&mut GpuVec<T>` binds each thread in `space` exclusively to a single element (`&mut T`), whereas a `Slice` provides unpartitioned indexed access.",
            ),
            Some(
                "pass the parent `GpuVec` directly for automatic per-cell distribution, or change the nam parameter type to an indexed slice `&mut [T]`.",
            ),
        )),

        1004 => Some(DiagnosticTemplate::error(
            1004,
            "mismatched container element type in nam dispatch",
            "container element type does not match parameter type",
            Some("parameter declared with this element type here"),
            Some(
                "every container argument passed to a nam must contain elements that strictly match the parameter's declared type.",
            ),
            Some(
                "ensure the container element type (e.g. `GpuVec<f32>`) strictly matches the nam parameter declaration.",
            ),
        )),

        1005 => Some(DiagnosticTemplate::error(
            1005,
            "zero-sized types are not permitted in GPU containers",
            "zero-sized element type used here",
            None,
            Some(
                "GPU parallel execution requires elements to possess a concrete physical footprint in memory.",
            ),
            Some(
                "ensure your struct contains at least one non-empty field (e.g. `u32`, `f32`, or a non-empty composite).",
            ),
        )),

        1006 => Some(DiagnosticTemplate::error(
            1006,
            "tile shared memory capacity exceeded",
            "requested tile scratchpad memory is too large",
            None,
            Some(
                "on-chip intra-tile shared scratchpad memory (`GpuTileMem`) is physically constrained per compute unit.",
            ),
            Some(
                "reduce the element count `N` in `GpuTileMem<T, N>` or use a more compact element type.",
            ),
        )),

        1007 => Some(DiagnosticTemplate::error(
            1007,
            "conflicting access to overlapping slices in nam dispatch",
            "mutable slice overlaps with an existing slice of the same GpuVec",
            Some("first slice access occurs here"),
            Some(
                "parallel threads executing in `space` cannot safely write to memory that is concurrently being accessed by other threads.",
            ),
            Some(
                "ensure sub-slices derived from the same parent GpuVec are disjoint using `.split_at_mut()` or non-intersecting element ranges.",
            ),
        )),

        1008 => Some(DiagnosticTemplate::error(
            1008,
            "GpuVec capacity is smaller than the requested space domain",
            "container contains fewer elements than required space cells",
            None,
            Some(
                "each cell in `space` expects exclusive 1:1 access to its corresponding element; extra threads would access out-of-bounds memory.",
            ),
            Some(
                "resize the `GpuVec` using `.resize(...)` to cover all space cells, or adjust the space domain dimensions.",
            ),
        )),

        1009 => Some(DiagnosticTemplate::error(
            1009,
            "unrestricted mutable slice access across parallel threads",
            "mutable slice permits unconstrained concurrent writes",
            None,
            Some(
                "safe dispatch `.run(...)` requires that thread writes are provably isolated; a shared `SliceMut` permits arbitrary concurrent writes to identical indices (data race hazard).",
            ),
            Some(
                "for automatic 1:1 per-cell safety, pass `&mut GpuVec<T>`, or if your algorithm coordinates writes manually, dispatch via: `unsafe { nam.run_unchecked(...) }`.",
            ),
        )),

        1010 => Some(DiagnosticTemplate::error(
            1010,
            "cannot mutably borrow GpuVec after queuing it for presentation",
            "mutable borrow occurs after presentation in the same flow",
            Some("resource queued for screen presentation here"),
            Some(
                "screen transfer copies (`.present(...)`) are deferred and executed at the very end of the flow; modifying the container afterwards will overwrite the intended screen output.",
            ),
            Some(
                "ensure `.present()` is the final operation performed on this container within the active flow.",
            ),
        )),

        _ => None,
    }
}
