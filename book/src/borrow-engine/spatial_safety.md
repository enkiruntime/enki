# Spatial Safety & Range Collisions (E1007, E1008)

The spatial borrow checker (`SpatialBorrowChecker`) validates that memory containers passed to an execution grid do not violate physical memory boundaries or overlap concurrently during execution.

---

## 1. Space Domain Bounds (`error[E1008]`)

In the 1:1 SPMD model, each thread in `Space` is mapped directly to its corresponding scalar element in a `GpuVec` (thread `i` accesses element `i`).

If the global execution domain requires more threads than the container contains elements, trailing threads would calculate out-of-bounds Buffer Device Addresses, causing a GPU page fault or invalid memory read.

### Dynamic Verification
Before submitting the dispatch, the runtime verifies:
```text
container.len() >= space.size_x * space.size_y * space.size_z
```


If the capacity is insufficient, execution halts with diagnostic **`error[E1008]`**:

```text
error[E1008]: GpuVec capacity is smaller than the requested space domain
  --> src/main.rs:51:23
   |
51 |         scale_vectors.run(&Space::gpu_x(N), &mut vec);
   |                       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ container contains fewer elements than required space cells
   |
   = note: each cell in `space` expects exclusive 1:1 access to its
           corresponding element; extra threads would access out-of-bounds
           memory.
   = note: argument 1 (`GpuVec<f32>`) contains only 1000 elements
   = note: space execution (dimensions: 10000x1x1) requires at least 10000
           elements (deficit of 9000 elements)
   = help: resize the `GpuVec` using `.resize(...)` to cover all space
           cells, or adjust the space domain dimensions.
```

---

## 2. Spatial Slice Disjointness (`error[E1007]`)

When multiple slices derived from the same physical root buffer are passed to a single nam dispatch, the runtime must verify that concurrent writes do not collide.

### The Interval Intersection Algorithm
For any pair of slices `A` and `B` sharing the same underlying allocation root ID:
- If both are immutable (`&[T]`), concurrent access is permitted.
- If either slice is mutable (`&mut [T]`), the engine computes their interval intersection:
```text
overlap_start = max(A_start, B_start)
overlap_end   = min(A_end, B_end)
```

If `overlap_start < overlap_end`, an overlapping memory range exists. The runtime halts execution with diagnostic **`error[E1007]`**:

```text
error[E1007]: conflicting access to overlapping slices in nam dispatch
  --> src/main.rs:20:24
   |
20 |         process_slices.run_unchecked(&Space::gpu_x(N), &mut slice_a, &slice_b);
   |                        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ mutable slice overlaps with an existing slice of the same GpuVec
   |
   = note: parallel threads executing in `space` cannot safely write to
           memory that is concurrently being accessed by other threads.
   = note: argument 1 covers elements [400..1000]
   = note: argument 2 covers elements [0..500]
   = note: spatial collision occurs on 100 overlapping elements [400..500]
           of the parent GpuVec<f32>
   = help: ensure sub-slices derived from the same parent GpuVec are
           disjoint using `.split_at_mut()` or non-intersecting element
           ranges.
```

### The Idiomatic Solution: `split_at_mut`
To guarantee spatial disjointness without runtime interval checking, use standard Rust partitioning methods such as `split_at_mut()`:

```rust
let mut vec = gpu_vec![2.0f32; 1000]; // parent vector
let mut output = gpu_vec![0.0f32; 1000]; // output vector

vec.copy_from_slice_at(500, &[10.0f32; 500]); // filling the parent vector with 10.0f32 after index 500.

let mut slice = vec.as_mut_slice(); // getting a mutable slice of the parent vector.

// `left` covers elements [0..500] and it holds the value of `2.0f32` and `right` covers elements [500..1000] and it holds the value of `10.0f32`
let (mut left, mut right) = slice.split_at_mut(500); // splitting the slice into two at index 500

process_slices.run_unchecked(&Space::gpu_x(N), &mut left, &mut right, &mut output);
```
