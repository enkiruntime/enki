# Divergent Control Flow & Barrier Deadlocks (`error[E0009]`)

One of the most catastrophic failure modes on graphics silicon is the **barrier deadlock**. Unlike CPU architectures where waiting on a synchronization primitive can timeout or throw exceptions, an invalid GPU memory barrier can hang the physical compute unit entirely, triggering an operating system Timeout Detection and Recovery (TDR) event or device reset.

This chapter details the physical execution model behind thread divergence and how Enki statically prevents barrier deadlocks at compile time.

---

## 1. Physical Warp Execution & Branch Divergence

GPU hardware schedules threads in lockstep groups called **Warps** (NVIDIA: 32 threads) or **Wavefronts** (AMD / Intel: 32 or 64 threads). All threads within a warp advance under a single shared program counter.

When code encounters a conditional branch:

```rust
if space.cell_x < 16 {
    do_something();
} else {
    do_other();
}
```

The hardware cannot physically branch in two different directions at the same clock cycle. Instead, it executes both sides sequentially through **thread masking**:
1. Active mask is set for cells `0..16`; other cells are disabled while `do_something()` executes.
2. Active mask is inverted for cells `16..32`; cells `0..16` are disabled while `do_other()` executes.

This serialization is known as **Branch Divergence**. While performance degrades, execution remains functionally correct for independent computation.

---

## 2. The Anatomy of a Barrier Deadlock

A synchronization barrier (`space.sync()`) lowers to a hardware instruction (`OpControlBarrier` in SPIR-V) with workgroup execution scope. 

The physical hardware contract of a barrier is absolute: **every active thread in the workgroup must execute the barrier instruction before any thread can proceed past it**.

### The Divergent Barrier Scenario
Consider what happens when a barrier is placed within a non-uniform branch:
 
```rust
// CRITICAL FAULT: Divergent Barrier
if space.cell_x < 128 {
    tile_cache[space.cell_x] = input[space.x];
    space.sync(); // Threads 0..127 wait here
} else {
    // Threads 128..255 bypass the barrier entirely!
}
```

In a workgroup of 256 threads:
1. Threads `0..127` enter the `if` block, reach `space.sync()`, and halt their execution units waiting for the remaining 128 threads to arrive.
2. Threads `128..255` take the `else` branch, proceed forward, and never issue the barrier instruction.
3. **The Deadlock:** Threads `0..127` wait indefinitely for signals that can never be sent. The physical hardware compute unit locks permanently.

---

## 3. The Compiler Invariant (`error[E0009]`)

To prevent device hangs, Enki's JIT compiler analyzes the control flow graph (CFG) of the `#[nam]` during bitcode canonicalization.

If an invocation of `space.sync()` is reachable from a non-uniform branch condition (any condition that evaluates differently across cells in the tile), compilation halts immediately:

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

## 4. The Trailing Thread Boundary Trap

A common mistake in GPU programming occurs when guarding problem boundaries:

```rust
// WRONG PATTERN: Early exit causes deadlock in boundary tiles
if !space.in_bounds_x() {
    return; // Trailing threads exit the kernel early!
}

tile_cache[space.cell_x] = input[space.x];
space.sync(); // DEADLOCK: Trailing threads never reach this barrier!
```

If the global problem size is not an exact multiple of the tile size (for instance, 1000 elements partitioned into tiles of 256):
- The final tile contains 232 valid elements and 24 inactive trailing threads.
- The 24 trailing threads execute `return` and terminate.
- The 232 active threads hit `space.sync()` and deadlock waiting for the terminated threads.

### The Correct Uniform Pattern
All threads within the tile must participate in the barrier. Clamp or zero out out-of-bounds memory loads instead of early-exiting:

```rust
// CORRECT: All threads reach the barrier unconditionally
let value = if space.in_bounds_x() {
    input[space.x]
} else {
    0.0 // Trailing threads load neutral dummy data
};

tile_cache[space.cell_x] = value;

// Uniform execution: all 256 cells execute the barrier together
space.sync();

if space.in_bounds_x() {
    output[space.x] = tile_cache[space.cell_x];
}
```

By ensuring that `space.sync()` is executed uniformly by all cells in the workgroup, the dispatch remains provably deadlock-free.
