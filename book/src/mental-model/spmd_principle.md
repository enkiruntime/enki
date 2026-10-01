# The SPMD Per-Cell Principle

Most GPU architectures operate on the **Single Program, Multiple Data (SPMD)** execution model. In this model, thousands of physical threads execute the same compiled instruction stream simultaneously, each operating on distinct data elements.

Enki leverages Rust's type system to express SPMD execution directly, eliminating manual thread-index calculations.

---

## 1. Traditional Indexing vs. Type-Level SPMD

In traditional GPU compute environments (CUDA, OpenCL, WGSL), every kernel receives raw array pointers and requires developers to manually compute linear thread coordinates:

### The Traditional Approach (CUDA / C99)
```c
__global__ void scale_kernel(float* in, float* out, float factor, int total_elements) {
    // Manual 1D grid stride calculation
    int i = threadIdx.x + blockDim.x * blockIdx.x;
    
    // Manual boundary guarding
    if (i < total_elements) {
        out[i] = in[i] * factor; // Unchecked raw array access
    }
}
```

This model places several burdens on the developer:
1. Arithmetic errors in the index calculation cause silent memory corruption or out-of-bounds faults.
2. The language has no concept of whether a thread has exclusive ownership of index `i` or if other threads are writing to the same location concurrently.

### The Enki Approach (Type-Level SPMD)
In Enki, you write the kernel as if it operates on a **single scalar element**:

```rust
#[nam]
fn scale_nam(_space: &Space, in_val: &f32, out_val: &mut f32, factor: f32) {
    *out_val = *in_val * factor;
}
```

When you dispatch this nam with `scale_nam.run(&Space::gpu_x(COUNT), &in_vec, &mut out_vec, GpuParam::new(factor))`:
- **Thread Isolation:** The JIT compiler and runtime lower `in_val: &f32` to address `base_in + (thread_id * 4)` and `out_val: &mut f32` to `base_out + (thread_id * 4)`.
- **Zero Manual Indexing:** Each thread receives an isolated reference to its own element.
- **Physical Safety by Construction:** Because thread `A` accesses element `A` and thread `B` accesses element `B`, concurrent write operations are spatially disjoint. Rust's rule—that a mutable reference `&mut T` represents exclusive access—is preserved across parallel silicon cores.

---

## 2. Per-Cell References vs. Indexed Slices

Not all GPU algorithms are strictly element-wise. Algorithms such as stencils, convolutions, physical simulations, and raymarchers require threads to inspect neighboring elements or traverse arbitrary memory locations.

Enki differentiates these access modes through Rust's container types:

| Rust Parameter in `#[nam]` | Access Pattern | Hardware Mapping | Primary Use Case |
| :--- | :--- | :--- | :--- |
| **`&T` / `&mut T`** | **Per-Cell SPMD:** Thread $i$ accesses element $i$. | Direct BDA offset per-thread. | Element-wise transforms, math, particle updates. |
| **`&[T]`** | **Global Slice Read:** Thread `i` can read any element `slice[k]`. | 16-byte Fat Pointer `(bda, count)`. | Reading lookup tables, neighboring cells, scene data. |
| **`&mut [T]`** | **Global Slice Write:** Thread `i` can write to any element `slice[k]`. | 16-byte Fat Pointer `(bda, count)`. | Arbitrary scatter writes, framebuffers, screen rendering. |

---

## 3. Dispatch Modes: `.run()` vs. `.run_unchecked()`

Enki provides two dispatch methods to balance safety and performance:

### Safe Mode (`.run()`)
Used for standard dispatches where data flow is element-wise (`&mut T`) or reads are shared (`&[T]`). In this mode, the runtime ensures that thread writes are provably isolated and mutually disjoint.

```rust
// Safe, automated 1:1 per-cell dispatch
scale_nam.run(&Space::gpu_x(COUNT), &in_vec, &mut out_vec, GpuParam::new(factor));
```

### Unchecked Mode (`.run_unchecked()`)
When your algorithm requires arbitrary index writes across a mutable slice (`&mut [T]`)—such as scattering particles into a screen buffer or custom tiled sorting—you dispatch using `unsafe { nam.run_unchecked(...) }`:

```rust
unsafe {
    render_nam.run_unchecked(
        &Space::gpu_x(COUNT),
        &particles,
        &mut screen_buffer.as_mut_slice(),
    );
}
```

The `unsafe` block explicitly documents that writes across the slice are coordinated manually by your algorithm, mirroring standard Rust systems programming semantics.
