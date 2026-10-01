# On-Chip Scratchpad Memory (`GpuTileMem`)

Modern graphics hardware features a hierarchy of memory tiers. While global VRAM provides high capacity, latency to access it remains relatively high. 

To hide memory latency in communication-heavy algorithms (such as stencils, convolutions, Fourier transforms, and pairwise interactions), GPU architectures include dedicated on-chip **Local Data Share (LDS) / Shared Memory (SRAM)** physically situated adjacent to the compute execution cores.

Enki exposes this hardware capability through **`GpuTileMem<T, N>`** and intra-tile synchronization barriers.

---

## 1. Architectural Characteristics

`GpuTileMem<T, N>` differs fundamentally from `GpuVec<T>` and `GpuParam<T>`:

- **Zero Allocation Footprint:** `GpuTileMem` does not allocate physical VRAM and incurs a 0-byte footprint in the engine's `ParamArena`. It simply informs the JIT compiler to allocate space in the workgroup's hardware shared storage registers.
- **Workgroup-Local Scope:** Memory allocated through `GpuTileMem` is shared exclusively among threads executing within the same spatial tile. Threads in different tiles cannot observe each other's tile memory.
- **Compile-Time Sizing:** The element capacity `N` must be a compile-time constant.

Inside a `#[nam]`, passing `&GpuTileMem<T, N>` binds as a mutable fixed-size array reference:

```rust
&mut [T; N]
```

---

## 2. Cooperative Tiling Pattern

The standard architectural pattern for shared memory involves cooperative loading, synchronization, and consumption:

```rust
use enki::*;

const TILE_SIZE: usize = 256;

#[nam]
fn tiled_convolution(
    space: &Space,
    global_input: &[f32],
    output: &mut f32,
    tile_cache: &mut [f32; TILE_SIZE], // On-chip scratchpad
) {
    // 1. Cooperative Load: Each thread in the tile loads one element into SRAM
    tile_cache[space.cell_x] = global_input[space.x];

    // 2. Intra-Tile Barrier: Wait for all 256 cells to complete writing
    space.sync();

    // 3. Compute: Read from shared memory without accessing global VRAM
    let left = if space.cell_x > 0 { tile_cache[space.cell_x - 1] } else { 0.0 };
    let center = tile_cache[space.cell_x];
    let right = if space.cell_x < TILE_SIZE - 1 { tile_cache[space.cell_x + 1] } else { 0.0 };

    *output = (left + center + right) / 3.0;
}
```

On the host, the dispatch must explicitly define a tile size matching the scratchpad capacity:

```rust
let tile_mem = GpuTileMem::<f32, TILE_SIZE>::new();

tiled_convolution.run(
    &Space::gpu_x(100_000).tile(TILE_SIZE),
    &input_slice,
    &mut output_vec,
    &tile_mem,
);
```

---

## 3. Control Flow Uniformity & Divergence Hazards (`error[E0009]`)

On GPU silicon, physical thread groups execute instructions in lockstep (SIMD / SIMT). A memory barrier (`space.sync()`) instructs the hardware execution unit to pause all threads in the workgroup until every thread reaches that barrier instruction.

### The Divergence Invariant
Because the barrier operates on the physical workgroup as a collective unit, **every cell in the tile must reach `space.sync()` unconditionally**.

If a barrier is placed inside a branch that evaluates differently across threads within the same tile (a non-uniform branch), the hardware enters a permanent deadlock: threads taking the branch pause waiting for non-branching threads, which will never arrive.

```rust
// COMPILE ERROR: Divergent tile synchronization
if space.cell_x < 128 {
    // Only half the tile reaches this barrier! Permanent GPU hang.
    space.sync();
}
```

This rule applies equally to boundary checks:

```rust
// COMPILE ERROR: Early exit before barrier
if !space.in_bounds_x() {
    return; // Trailing threads exit, leaving active threads hung at space.sync()
}

tile_cache[space.cell_x] = global_input[space.x];
space.sync();
```

Enki's JIT compiler analyzes the control flow graph (CFG) for barrier reachability. If a barrier is found within non-uniform conditional blocks, compilation is rejected immediately:

```text
error[E0009]: divergent tile synchronization detected inside #[nam]
  --> src/main.rs:18:9
   |
15 |     if space.cell_x < 128 {
   |        ------------------ branch condition is non-uniform across tile cells
...
18 |         space.sync();
   |         ^^^^^^^^^^^^ tile synchronization called here
   |
   = note: all cells in a tile must reach `space.sync()` concurrently; barriers inside non-uniform control flow cause permanent GPU hardware deadlocks.
   = help: move `space.sync()` outside the conditional block or ensure the branch condition is uniform across the entire tile.
```

---

## 4. Hardware Capacity Verification (`error[E1006]`)

Physical GPU compute units have strict hardware limits on total shared memory capacity (typically 32 KB, 48 KB, or 64 KB per workgroup depending on the device architecture).

The BorrowEngine verifies requested capacity (`N * size_of::<T>()`) against the queried hardware profile prior to execution:


```text
error[E1006]: tile shared memory capacity exceeded
  --> src/main.rs:12:9
   |
12 |         kernel.run(&space, &large_tile_mem);
   |         ^^^^^^ requested tile scratchpad memory is too large
   |
   = note: on-chip intra-tile shared scratchpad memory (`GpuTileMem`) is physically constrained per compute unit.
   = help: reduce the element count `N` in `GpuTileMem<T, N>` or use a more compact element type.
```

---

## 5. CPU Simulation Boundaries

As outlined in the introduction, `GpuTileMem` and `Space::sync()` represent an area where standard CPU iteration diverges from GPU execution.

When executing on silicon, threads run cooperatively and communicate through hardware memory fences. On the host, a standard sequential loop (`for i in 0..N`) executes iterations strictly one after another. Iteration `0` completes entirely before iteration `1` begins; therefore, calling `space.sync()` on a CPU thread simply performs a host compiler fence (`compiler_fence`) without suspending execution to wait for other iterations.

Accurately simulating cooperative workgroup barriers and shared memory on host CPU threads requires complex compiler transformations (such as loop splitting or fiber coroutine scheduling), which remain an active research track within Enki.
