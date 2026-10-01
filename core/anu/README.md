# enki-anu

Core GPU compute execution engine, task recording queue, and compile-time BorrowEngine for Enki.
Part of the [Enki](https://github.com/enkiruntime/enki) ecosystem.

## Features
- GPU BorrowEngine: Prevents spatial and temporal data race hazards across parallel dispatches
- Transitive Reduction Solver: Automatically derives optimal Vulkan pipeline barriers (`Synchronization2`)
- JIT Synthesis Coordinator: Manages just-in-time compilation of host Rust nams into GPU pipelines
- Diagnostic Engine: error reporting for GPU runtime invariants
