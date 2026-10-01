# Memory & Resource System

GPU hardware operates on distinct physical memory tiers with differing latency, bandwidth, and scope characteristics:

- **Global VRAM (High Bandwidth Memory):** Dedicated device memory accessible by all GPU compute units.
- **ParamArena (Uniform Memory):** High-speed linear payload space used for passing configurations and addresses into shader registers.
- **On-Chip Shared Memory (SRAM / LDS):** Ultra-low-latency scratchpad memory physically local to each compute workgroup.

Enki mirrors these hardware tiers through five distinct Rust abstractions:

| Type | Physical Memory Tier | Ownership & Lifecycle | Primary Use Case |
| :--- | :--- | :--- | :--- |
| **`GpuVec<T>`** | Global VRAM | Owned allocation via 64-bit BDA. Moves and deep-clones. | Large data sets, vertex arrays, simulation state. |
| **`Slice<T>` / `SliceMut<T>`** | Global VRAM | Borrowed zero-allocation sub-range views. | Sub-slicing without re-allocating VRAM. |
| **`GpuParam<T>`** | ParamArena (Uniforms) | Packed by-value into linear dispatch payload. | Small-to-medium structs (`Camera`, `Config`). |
| **`GpuAtomic<T>`** | Global VRAM | Dedicated device storage with hardware atomic instructions. | Global counters, locks, parallel compaction. |
| **`GpuTileMem<T, N>`** | On-Chip SRAM (LDS) | Zero allocation. Allocated in local workgroup registers. | Cooperative tile caching, intra-tile reductions. |

---

## Chapter Overview

- **[Owned VRAM with `GpuVec`](gpu_vec.md):** Allocation, capacity management, physical cloning, and timeline synchronization.
- **[Zero-Cost Slicing (`Slice` & `SliceMut`)](slices.md):** View creation, BDA address arithmetic, and disjoint mutable splitting.
- **[By-Value Uniforms (`GpuParam`)](uniforms_param.md):** Packing arbitrary structs and configuration types into the uniform arena.
- **[Hardware Atomics (`GpuAtomic`)](atomics.md):** Managing concurrent scalar and vector atomics across GPU threads.
- **[On-Chip Scratchpad (`GpuTileMem`)](tile_memory.md):** Leveraging local workgroup SRAM and coordinating access with `Space::sync()`.
