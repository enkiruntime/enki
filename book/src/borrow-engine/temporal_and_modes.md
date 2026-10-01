# Temporal Safety & Dispatch Modes (E1010)

In addition to validating spatial boundaries within a single dispatch, the `BorrowEngine` tracks resource states chronologically across the entire active frame via the **`FrameBorrowLedger`**.

---

## 1. Temporal Presentation Hazards (`error[E1010]`)

In real-time graphics and display pipelines, presenting a buffer to the screen is an asynchronous operation. When you call:

```rust
flow.present(&pixel_buffer);
```

Enki does not immediately copy the pixels to the swapchain. Instead, it marks the buffer as queued for display in the active command buffer. The physical transfer occurs at the conclusion of the frame submission pipeline.

### The Mutation Hazard
If subsequent dispatches within the same flow attempt to mutate the buffer after queuing it for presentation:

```text
Frame Execution Timeline (Recording Phase)
──[nam: processing `pixels`]──► [flow.present(pixels)] ──► [nam: Clear Image (&mut pixels)] ──► [Submit]
                                                            HAZARD: E1010
                                                            The intended presentation output would be corrupted!
```

Because the GPU executes the recorded commands in order, mutating the buffer after calling `flow.present()` would overwrite the image before the swapchain copy command can read it.

For example, if you write this pattern of code:

```rust
enki.flow(|flow| {
    process_image.run(&Space::gpu_xy(W, H), &left, &right, &mut output);
    flow.present(&output);
    clear_image.run(&Space::gpu_xy(W, H), &mut output);
});
```

The `FrameBorrowLedger` records presentation queues and halts execution with diagnostic **`error[E1010]`**:

```text
error[E1010]: cannot mutably borrow GpuVec after queuing it for presentation
  --> src/main.rs:31:21
   |
31 |         clear_image.run(&Space::gpu_xy(W, H), &mut output);
   |                     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ mutable borrow occurs after presentation in the same flow
   |
   = note: screen transfer copies (`.present(...)`) are deferred and executed at the very end of the flow; modifying the
           container afterwards will overwrite the intended screen output.
   = note: argument 1 (parent GpuVec #5) was already queued for screen presentation via `.present()` earlier in this
           flow
   = help: ensure `.present()` is the final operation performed on this container within the active flow.
```

---

## 2. Dispatch Modes: Safe vs. Unchecked

Enki provides two execution modes to accommodate different algorithmic requirements:

### Safe Mode (`.run()`)
Safe mode is the default dispatch path. It enforces the following invariants:
- **1:1 SPMD Isolation:** Guarantees that thread `i` writes exclusively to cell `i`.
- **Slice Disjointness:** Rejects dispatches with overlapping mutable intervals (`E1007`).
- **Domain Coverage:** Rejects dispatches where container capacities do not satisfy the problem space (`E1008`).
- **Presentation Immutability:** Rejects mutable borrows on presented buffers (`E1010`).

### Unchecked Mode (`.run_unchecked()`)
Certain algorithms require arbitrary or indirect write operations across an entire mutable slice (`&mut [T]`)—such as software rasterization, particle splatting, or custom parallel radix sorting.

In such cases, the compiler cannot statically or dynamically prove that concurrent writes will not collide. You explicitly opt out of safe mode:

```rust
unsafe {
    scatter_kernel.run_unchecked(&space, &mut global_slice);
}
```

### What `run_unchecked` Disables (and What it Retains)
- **Disabled:** It bypasses restrictions on passing unrestricted mutable slices across parallel threads.
- **Retained:** It does **not** disable all runtime validation. Spatial domain checks, interval collision detection, and temporal presentation tracking remain actively enforced by the `BorrowEngine`.

The `unsafe` block explicitly signifies that spatial race-freedom inside the slice is guaranteed by the developer's indexing algorithm.
