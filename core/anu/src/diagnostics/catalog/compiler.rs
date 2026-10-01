use super::entry::DiagnosticTemplate;

pub fn lookup(code: u32) -> Option<DiagnosticTemplate> {
    match code {
        1 => Some(DiagnosticTemplate::error(
            1,
            "unsupported CPU system operation inside #[nam]",
            "unsupported CPU call occurs here",
            None,
            Some(
                "GPU nams execute isolated on graphics silicon without access to CPU OS services (I/O, filesystem, threads).",
            ),
            Some(
                "perform CPU operations in the main thread and transfer data using `GpuVec` or `GpuParam`.",
            ),
        )),

        2 => Some(DiagnosticTemplate::error(
            2,
            "dynamic heap allocation is not supported inside #[nam]",
            "heap allocation occurs here",
            None,
            Some(
                "GPU nams execute in parallel across silicon cells without a dynamic memory heap manager.",
            ),
            Some(
                "use fixed-size stack arrays `[T; N]` or pre-allocate a `GpuVec` on the CPU before dispatch.",
            ),
        )),

        3 => Some(DiagnosticTemplate::error(
            3,
            "explicit panic or unwinding assertion inside #[nam]",
            "panic or assertion occurs here",
            None,
            Some(
                "GPU silicon lacks stack unwinding and landing pad machinery; execution must be non-panicking.",
            ),
            Some(
                "avoid panicking APIs (e.g. `.unwrap()`, `assert!`) and handle bounds via conditional checks.",
            ),
        )),

        4 => Some(DiagnosticTemplate::error(
            4,
            "unsupported data type on GPU silicon",
            "unsupported type used here",
            None,
            Some(
                "types larger than 64 bits (such as `i128`, `u128`, or `f128`) have no native registers on GPU hardware.",
            ),
            Some("use 32-bit or 64-bit primitive types (`u32`, `i32`, `f32`, `u64`, `i64`)."),
        )),

        5 => Some(DiagnosticTemplate::error(
            5,
            "recursion is not supported inside #[nam]",
            "recursive call occurs here",
            Some("function defined here"),
            Some(
                "GPU nams execute on fixed registers without a dynamic call stack to allocate new data at runtime.",
            ),
            Some("rewrite recursive algorithms using iterative loops (`for` / `while`)."),
        )),

        6 => Some(DiagnosticTemplate::error(
            6,
            "inline assembly is not supported inside #[nam]",
            "inline assembly instruction used here",
            None,
            Some("CPU architecture assembly instructions cannot run on GPU compute pipelines."),
            Some(
                "use Enki space intrinsics (`space.sync()`, `space.x`, etc.) for hardware operations.",
            ),
        )),

        7 => Some(DiagnosticTemplate::error(
            7,
            "mutable global state is not permitted in GPU execution",
            "mutable global access occurs here",
            None,
            Some(
                "static mutable variables create unresolvable race conditions across concurrent GPU workgroups.",
            ),
            Some(
                "pass mutable buffers explicitly via `&mut GpuVec` or synchronize through `GpuAtomic`.",
            ),
        )),

        8 => Some(DiagnosticTemplate::error(
            8,
            "GPU nam function not found in compiled bitcode",
            "nam function entry point not found",
            None,
            Some("the compiler was unable to isolate an exported symbol matching the target nam."),
            Some(
                "ensure the function is annotated with `#[nam]` above its signature (e.g. `#[nam] fn my_nam(space: &Space, ...)`).",
            ),
        )),

        9 => Some(DiagnosticTemplate::error(
            9,
            "divergent tile synchronization detected inside #[nam]",
            "tile synchronization called here",
            Some("branch condition is non-uniform across tile cells"),
            Some(
                "all cells in a tile must reach `space.sync()` concurrently; barriers inside non-uniform control flow cause permanent GPU hardware deadlocks.",
            ),
            Some(
                "move `space.sync()` outside the conditional block or ensure the branch condition is uniform across the entire tile.",
            ),
        )),

        10 => Some(DiagnosticTemplate::error(
            10,
            "dynamic dispatch and indirect calls are not supported inside #[nam]",
            "indirect function call occurs here",
            None,
            Some(
                "GPU pipelines require static control flow and cannot invoke functions through runtime pointers or vtables (`&dyn Trait`).",
            ),
            Some(
                "use concrete types, static generics (`impl Trait`), or an `enum` with pattern matching (`match`).",
            ),
        )),

        11 => Some(DiagnosticTemplate::error(
            11,
            "native LLVM bitcode artifact is missing for JIT compilation",
            "required .bc bitcode artifact is missing from target directory",
            None,
            Some(
                "Enki requires LLVM bitcode (.bc) emitted by rustc to compile nams to GPU SPIR-V.",
            ),
            Some("please run your project using `enki run` instead of `cargo run`."),
        )),

        12 => Some(DiagnosticTemplate::error(
            12,
            "Enki GPU JIT compiler toolchain is missing",
            "required GPU compiler backend was not found",
            None,
            Some(
                "Enki requires the Parsu compilation backend to lower Rust nams into GPU machine code.",
            ),
            Some(
                "allow Enki to download the toolchain automatically on launch, or set `PARSU_LIB_PATH`.",
            ),
        )),

        42 => Some(DiagnosticTemplate::error(
            42,
            "cpu runtime management invoked inside `#[nam]`",
            "cpu-side operation cannot execute inside a cell",
            None,
            Some("a single cell in space cannot run an operating system."),
            Some(
                "allocate buffers and configure flows in `fn main()`, then pass them as parameters.",
            ),
        )),

        9999 => Some(DiagnosticTemplate::error(
            9999,
            "internal compiler error (ICE) in Parsu GPU backend (v0.1.0-alpha)",
            "compiler hit an unexpected invariant condition",
            None,
            Some("this is a known limitation of the early alpha release on some architectures."),
            Some(
                "we would appreciate a bug report! Please open an issue and include your #[nam] code along with the diagnostic token below:\nhttps://github.com/enkiruntime/enki/issues",
            ),
        )),

        _ => None,
    }
}
