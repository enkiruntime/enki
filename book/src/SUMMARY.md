# Summary

[Introduction](README.md)

- [Getting Started](getting-started/README.md)
  - [Hardware & Toolchain Setup](getting-started/installation.md)
  - [Your First Nam & Dual Debugging](getting-started/hello_gpu.md)

- [The Mental Model](mental-model/README.md)
  - [Rust References to 64-bit BDA](mental-model/references_to_bda.md)
  - [The SPMD Per-Cell Principle](mental-model/spmd_principle.md)
  - [Spatial Execution & Topology (`Space`)](mental-model/space_topology.md)

- [Memory & Resource System](resources/README.md)
  - [Owned VRAM with `GpuVec`](resources/gpu_vec.md)
  - [Zero-Cost Slicing (`Slice` & `SliceMut`)](resources/slices.md)
  - [By-Value Uniforms with `GpuParam`](resources/uniforms_param.md)
  - [Hardware Atomics (`GpuAtomic`)](resources/atomics.md)
  - [On-Chip Scratchpad (`GpuTileMem`)](resources/tile_memory.md)

- [Compiler Invariants & Silicon Rules](compiler-invariants/README.md)
  - [What Cannot Run on GPU (E0001 - E0010)](compiler-invariants/rejected_patterns.md)
  - [Divergence & Tile Synchronization (E0009)](compiler-invariants/divergent_control_flow.md)
  - [Internal Compiler Errors & Reporting (E9999)](compiler-invariants/crash_flight_recorder.md)

- [The GPU BorrowEngine](borrow-engine/README.md)
  - [Spatial Bounds & Range Collisions (E1007, E1008)](borrow-engine/spatial_safety.md)
  - [Temporal Presentation & Dispatch Modes (E1010)](borrow-engine/temporal_and_modes.md)

- [Graphics & The Barrier Engine](graphics-and-engine/README.md)
  - [Real-Time Presentation (`flow.present`)](graphics-and-engine/presentation_pipeline.md)
  - [The Automatic Barrier Solver (TTRD)](graphics-and-engine/ttrd_barrier_solver.md)
