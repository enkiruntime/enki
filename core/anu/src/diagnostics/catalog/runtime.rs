use super::entry::DiagnosticTemplate;

pub fn lookup(code: u32) -> Option<DiagnosticTemplate> {
    match code {
        2001 => Some(DiagnosticTemplate::error(
            2001,
            "illegal host readback during GPU command recording phase",
            "host readback attempted here",
            None,
            Some(
                "dispatches inside `enki.flow` are recorded into a command buffer and have not been submitted to the GPU; waiting for results here causes a permanent CPU-GPU deadlock.",
            ),
            Some(
                "move readback operations (`.to_vec()`, `.get()`, or `println!`) outside the `enki.flow` closure.",
            ),
        )),

        2002 => Some(DiagnosticTemplate::error(
            2002,
            "swapchain present dimension mismatch",
            "buffer presented with insufficient element count",
            None,
            Some(
                "the window extent requires a 1:1 pixel mapping for swapchain transfer operations.",
            ),
            Some(
                "resize or reallocate your `GpuVec` to match the window dimensions (`width * height`).",
            ),
        )),

        2003 => Some(DiagnosticTemplate::error(
            2003,
            "nam dispatched outside an active flow context",
            "nam dispatch attempted here",
            None,
            Some(
                "GPU execution requires an active command buffer recording context to schedule dispatches and timeline barriers.",
            ),
            Some("wrap your GPU dispatches inside an `enki.flow(|flow| { ... })` closure."),
        )),

        2004 => Some(DiagnosticTemplate::error(
            2004,
            "nested flow recording is not permitted",
            "nested `enki.flow` invoked here",
            None,
            Some(
                "flow command recording is non-reentrant on a single thread to guarantee deterministic timeline semaphore synchronization.",
            ),
            Some("combine your GPU dispatches sequentially within a single `enki.flow` block."),
        )),

        2005 => Some(DiagnosticTemplate::error(
            2005,
            "GPU slice index out of bounds",
            "invalid slice range specified here",
            None,
            Some("GPU slice bounds must reside strictly within the allocated VRAM buffer limits."),
            Some("verify that range indices satisfy `start <= end` and `end <= slice.len()`."),
        )),

        2006 => Some(DiagnosticTemplate::error(
            2006,
            "GPU timeline synchronization timeout",
            "timeline wait timed out here",
            None,
            Some(
                "the GPU failed to advance the timeline semaphore counter within the watchdog threshold (possible GPU hang).",
            ),
            Some("inspect your `#[nam]` for infinite loops or reduce compute grid dimensions."),
        )),

        _ => None,
    }
}
