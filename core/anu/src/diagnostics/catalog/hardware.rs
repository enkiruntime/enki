use super::entry::DiagnosticTemplate;

pub fn lookup(code: u32) -> Option<DiagnosticTemplate> {
    match code {
        3000 => Some(DiagnosticTemplate::error(
            3000,
            "no compatible Vulkan graphics driver or runtime found",
            "Vulkan loader failed to locate an installable client driver (ICD)",
            None,
            Some(
                "Enki requires a functioning Vulkan 1.3+ runtime environment to discover graphics silicon and schedule compute tasks.",
            ),
            Some(
                "install the latest official graphics drivers for your GPU, or install the system Vulkan runtime.",
            ),
        )),

        3001 => Some(DiagnosticTemplate::error(
            3001,
            "GPU thread stack overflow",
            "failed to allocate GPU thread stack memory",
            None,
            Some(
                "each cell in the dispatch space requires local stack memory allocated in VRAM; the requested total stack exceeded available memory.",
            ),
            Some(
                "reduce local stack array sizes inside the `#[nam]` function or lower the spatial grid dimensions.",
            ),
        )),

        3002 => Some(DiagnosticTemplate::error(
            3002,
            "out of GPU device memory (VRAM exhaustion)",
            "VRAM allocation failed here",
            None,
            Some(
                "the physical graphics memory is exhausted and cannot satisfy the requested buffer allocation.",
            ),
            Some(
                "reduce GpuVec capacities, free unused GPU resources, or process data in streaming chunks.",
            ),
        )),

        3003 => Some(DiagnosticTemplate::warning(
            3003,
            "custom tile size exceeds GPU hardware invocations limit",
            "tile configuration exceeds maximum invocations (clamped to hardware limit)",
            None,
            Some(
                "GPU silicon enforces a strict hardware limit on the total cells per tile (`tile_x * tile_y * tile_z`), typically 1024 cells.",
            ),
            Some(
                "reduce custom tile dimensions or don't use `.tile(..)` to let Enki derive the mathematically optimal tile size.",
            ),
        )),

        3004 => Some(DiagnosticTemplate::error(
            3004,
            "Vulkan physical device lost",
            "fatal GPU hardware execution fault occurred here",
            None,
            Some(
                "the GPU driver encountered an unrecoverable hardware fault or an OS Timeout Detection and Recovery (TDR) trigger.",
            ),
            Some(
                "inspect your `#[nam]` for infinite loops, out-of-bounds memory accesses, or missing synchronization barriers.",
            ),
        )),

        3005 => Some(DiagnosticTemplate::error(
            3005,
            "missing required Vulkan compute features",
            "hardware compute capability check failed",
            None,
            Some(
                "Enki's execution model strictly relies on 64-bit Buffer Device Addresses (BDA), Timeline Semaphores, and Synchronization2 for memory safety and zero-overhead dispatch.",
            ),
            Some(
                "verify that your graphics hardware supports Vulkan 1.3+ and that your graphics drivers are up to date.",
            ),
        )),

        3006 => Some(DiagnosticTemplate::error(
            3006,
            "cannot create display window surface in headless environment",
            "window surface creation failed here",
            None,
            Some(
                "an active display server (Wayland, X11, or Windows DWM) is required to present buffers to a screen; no display handle was found.",
            ),
            Some(
                "if running on a remote server, CI/CD runner, or container, initialize Enki in headless mode using `Enki::init()` instead of `Enki::init_windowed()`.",
            ),
        )),

        3007 => Some(DiagnosticTemplate::error(
            3007,
            "no physical GPU devices enumerated by Vulkan",
            "zero physical devices available on the system",
            None,
            Some(
                "the Vulkan instance initialized successfully, but `enumerate_physical_devices` returned an empty list.",
            ),
            Some(
                "ensure your GPU is seated properly, enabled in the operating system device manager, and that container GPU passthrough is active.",
            ),
        )),

        3008 => Some(DiagnosticTemplate::warning(
            3008,
            "running on integrated GPU silicon",
            "integrated GPU selected for compute workload",
            None,
            Some(
                "an integrated GPU was selected while a high-performance discrete GPU might be available on this system.",
            ),
            Some(
                "configure your OS graphics preferences or set `DRI_PRIME=1` / `VK_ICD_FILENAMES` to ensure the high-performance discrete GPU is utilized.",
            ),
        )),

        3009 => Some(DiagnosticTemplate::warning(
            3009,
            "requested ParamArena size exceeds device allocation limit",
            "allocation clamped to hardware limit",
            None,
            Some(
                "the configured arena capacity is larger than the GPU's maximum single-allocation buffer limit.",
            ),
            Some(
                "lower `.param_arena_size(...)` on `Enki::builder()`, or pass large data by reference (`&GpuVec` or `&[T]`).",
            ),
        )),

        3010 => Some(DiagnosticTemplate::error(
            3010,
            "nam parameters payload exceeds ParamArena capacity",
            "parameter payload exceeds allocated arena capacity",
            None,
            Some(
                "the total byte footprint of by-value parameters (`GpuParam`) passed to this nam exceeds the pre-allocated capacity of the ParamArena.",
            ),
            Some(
                "increase the arena size using `Enki::builder().param_arena_size(...)` or pass large arrays by reference (`&GpuVec` or `&[T]`) instead of by value.",
            ),
        )),

        3011 => Some(DiagnosticTemplate::error(
            3011,
            "maximum GPU timestamp queries exceeded",
            "timestamp query limit reached here",
            None,
            Some(
                "each `flow.mark()` consumes one hardware query slot from the active flow query pool.",
            ),
            Some(
                "increase the query pool capacity during engine initialization using `Enki::builder().max_timestamp_queries(...)`.",
            ),
        )),

        _ => None,
    }
}
