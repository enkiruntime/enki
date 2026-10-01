# The GPU BorrowEngine: Two-Tier Safety & Research Scope

Rust's compile-time borrow checker statically guarantees memory safety on CPU threads by enforcing the aliasing XOR mutability invariant: memory may have multiple shared readers (`&T`), or exactly one exclusive writer (`&mut T`), but never both simultaneously.

Enki does not replace or bypass this mechanism. Instead, it is architected around a **Two-Tier Cooperative Defense Model**:
1. **Tier 1 (Host Compile-Time):** Leverages `rustc`'s native borrow checker to intercept aliasing bugs on the host CPU.
2. **Tier 2 (Device Runtime):** The `BorrowEngine` evaluates GPU-specific invariants where the host compiler has zero visibility.

---

## 1. Tier 1: Leveraging the Native Rust Borrow Checker

Enki's resource APIs are designed with standard Rust lifetime signatures. For example, obtaining a mutable slice from a `GpuVec` requires an exclusive borrow of the parent container:

```rust
impl<T> GpuVec<T> {
    pub fn slice_mut<R: RangeBounds<usize>>(&mut self, range: R) -> SliceMut<'_, T>;
    pub fn slice<R: RangeBounds<usize>>(&self, range: R) -> Slice<'_, T>;
}
```

Because `slice_mut` borrows `&mut self`:

```rust
let mut vec = gpu_vec![0.0f32; 1000];

let mut slice_a = vec.slice_mut(0..600);
//                                   ^^^ mutable borrow occurs here
let slice_b = vec.slice(600..1000);
//                            ^^^ cannot borrow `vec` as immutable because it is also borrowed as mutable
my_nam.run(&Space::gpu_x(N), &mut slice_a, &slice_b);
//                                       ^^^^^^^^^^^^ mutable borrow later used here
```

`rustc` intercepts this attempt **at compile time** inside the developer's editor. Enki deliberately relies on the host compiler as its first line of defense to eliminate trivial aliasing hazards before compilation even completes.

---

## 2. Tier 2: The Three Blind Spots of the Host Compiler

While `rustc` manages CPU references, it is physically unaware of graphics hardware architecture, execution grids, and deferred command queues.

The **`BorrowEngine`** intervenes at the dispatch boundary (`.run()`) to enforce invariants across three specific blind spots:

### Blind Spot A: Spatial Execution Domains (`error[E1008]`)
`rustc` verifies that a `GpuVec` is allocated and valid. However, the host compiler cannot inspect the dynamic dimensions of a GPU execution grid (`Space::gpu_x(1024)`).

If a buffer containing 512 elements is dispatched over 1024 threads in a 1:1 SPMD kernel:
- `rustc` considers the types valid and allows compilation.
- The GPU would attempt to calculate out-of-bounds Buffer Device Addresses for threads 512 through 1023.
- The `BorrowEngine` intercepts this at dispatch time and halts execution with `error[E1008]`.

### Blind Spot B: Temporal Presentation Latency (`error[E1010]`)
In Vulkan, presentation is deferred: calling `flow.present(&buffer)` records a transfer command that executes asynchronously at the conclusion of the frame submission pipeline.

To `rustc`:
- `flow.present(&buffer)` takes a shared read-only reference `&buffer` whose lifetime ends immediately after the statement.
- Subsequent calls like `clear.run(&space, &mut buffer)` inside the same recording block are permitted by `rustc`.

To the GPU:
- Modifying the buffer after queuing it for display would overwrite the image before the swapchain copy command can read it.
- The `BorrowEngine`'s `FrameBorrowLedger` tracks this chronological lifecycle and rejects the subsequent mutation with `error[E1010]`.

### Blind Spot C: Unchecked Slice Intersections (`error[E1007]`)
Certain advanced algorithms require passing multiple sub-slices derived from the same parent buffer. To permit this on the host, Enki provides explicit unchecked constructors that bypass host exclusivity:

```rust
// Bypasses host exclusivity by taking &self
let mut slice_a = unsafe { buffer.slice_mut_unchecked(0..600) };
let slice_b = buffer.slice(400..1000);
```

While `rustc` permits this because of the `unsafe` block, the **`BorrowEngine` does not trust the developer blindly**. 

Before command buffers are submitted to the GPU, the engine computes the mathematical interval intersection:
```text
overlap = [0..600) ∩ [400..1000) = [400..600)
```


If an overlap is detected on a mutable range, the `BorrowEngine` halts execution with `error[E1007]`.

---

## 3. Engineering Status: Pragmatic Safety vs. Formal Proofs

It is essential to understand the scientific boundaries of this architecture:

### An Active Systems Research Module
The `BorrowEngine` is an evolving component of the Enki runtime. Its algorithms and verification tables are actively being refined, expanded, and stress-tested.

### Deterministic Invariant Checking, Not Formal Soundness
As outlined before, Enki does **not** claim to provide a formally verified, mathematically sound type system for arbitrary parallel execution.

Formal verification (such as theorem proving in Coq or formal memory models like CompCert) mathematically proves that all possible program paths are sound. The `BorrowEngine`, by contrast, is a **compiler-grade pragmatic runtime guardrail**. It intercepts known, concrete classes of spatial and temporal data race hazards at the dispatch boundary, providing a practical safety net without claiming mathematical infallibility.

---

## Chapter Navigation

- **[Spatial Bounds & Range Collisions (E1007, E1008)](spatial_safety.md):** Detailed analysis of interval intersection math and space domain capacity checks.
- **[Temporal Presentation & Dispatch Modes (E1010)](temporal_and_modes.md):** Chronological frame tracking and the operational boundaries between Safe Mode and Unchecked Mode.
