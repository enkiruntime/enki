# Zero-Cost Slicing (`Slice` & `SliceMut`)

While `GpuVec<T>` owns physical allocations in VRAM, algorithms frequently need to operate on sub-ranges of data (such as image tiles, particle partitions, or sub-matrices) without copying memory.

Enki provides **`Slice<'a, T>`** (read-only) and **`SliceMut<'a, T>`** (exclusive mutable) to enable zero-allocation, borrowed sub-views over existing GPU buffers.

```rust
use enki::*;

let mut buffer = gpu_vec![0u32; 1024];

// Zero-allocation borrowed sub-views
let first_half: Slice<u32> = buffer.slice(..512);
let mut second_half: SliceMut<u32> = buffer.slice_mut(512..);
```

---

## 1. Zero-Cost Physical Address Arithmetic

Unlike traditional graphics APIs where creating a sub-buffer requires creating new Vulkan buffer views (`VkBufferView`) or allocating fresh descriptor sets, slicing in Enki is a purely mathematical **zero-cost abstraction**:

```text
Parent GpuVec (Base BDA: 0x7F00_0000, Stride: 4 bytes)
┌──────────────────────────────────────┬──────────────────────────────────────┐
│ Elements [0..512]                    │ Elements [512..1024]                 │
└──────────────────────────────────────┴──────────────────────────────────────┘
 ▲                                      ▲
 │                                      │
 Slice A: 0x7F00_0000                   Slice B: 0x7F00_0000 + (512 * 4)
 (Device Address: 0x7F00_0000)          (Device Address: 0x7F00_0800)
```

When you slice a container:
```text
device_address = parent_bda + (element_offset * stride)
```

Creating, cloning, or passing a slice performs zero GPU allocations, zero syscalls, and zero driver submissions. It simply passes a 16-byte fat pointer `(bda: u64, count: u64)` into the dispatch ingress.

---

## 2. Immutable Slices (`Slice<'a, T>`)

`Slice<'a, T>` represents a read-only view into a sub-range of GPU memory:

- **Implements `Clone`:** Multiple read-only slices derived from the same parent buffer can freely overlap and exist simultaneously.
- **Kernel Binding:** In a `#[nam]` function, passing `&Slice<T>` binds as an indexed global slice:
  ```rust
  #[nam]
  fn read_lookup(_space: &Space, table: &[f32], target: &mut f32) {
      *target = table[42]; // Free random read access
  }
  ```

### Slicing Sub-ranges
Slices can be recursively partitioned into smaller sub-views:

```rust
let view = buffer.slice(100..500);
let sub_view = view.slice(0..50); // Covers elements 100..150 of parent
let (left, right) = view.split_at(200);
```

---

## 3. Mutable Slices (`SliceMut<'a, T>`)

`SliceMut<'a, T>` enforces Rust's exclusive write semantics over a VRAM sub-range:

- **Does NOT implement `Clone`:** Enforces single-writer exclusivity.
- **Reborrowing:** A `SliceMut` can be reborrowed as an immutable `Slice` via `.as_slice()` or consumed via `.into_slice()`.
- **Kernel Binding:** In a `#[nam]`, passing `&mut SliceMut<T>` binds as a global read-write slice `&mut [T]`.

### Disjoint Partitioning with `split_at_mut`
To safely divide a mutable buffer into two independent mutable slices, use `split_at_mut`:

```rust
let mut buffer = gpu_vec![0.0f32; 1000];
let mut slice = buffer.as_mut_slice();

// Guarantees mathematically disjoint ranges [0..500) and [500..1000)
let (mut left, mut right) = slice.split_at_mut(500);
```

---

## 4. Host Manipulation Methods

Both slice types support direct data inspection and modification from the host CPU. Like `GpuVec`, host readbacks automatically wait on the GPU timeline if the underlying buffer is in-flight:

```rust
let mut slice = buffer.slice_mut(0..100);

// Set single element
slice.set(0, 42.0);

// Bulk upload from CPU slice
let host_data = vec![1.0; 100];
slice.copy_from_slice(&host_data);

// Read back to host Vec
let cpu_copy: Vec<f32> = slice.to_vec();

// Clone into independent physical VRAM allocation
let separate_gpu_vec: GpuVec<f32> = slice.to_gpu_vec();
```

> **Phase Rule:** Slices track the execution phase of their underlying physical allocation. Calling `.to_vec()` or `.copy_from_slice()` on a slice inside an active `enki.flow` block halts execution with `error[E2001]`.

---

## 5. Runtime Bounds Verification (`error[E2005]`)

Attempting to slice beyond the element capacity of a container halts execution before command buffers are recorded:

```text
error[E2005]: GPU slice index out of bounds
  --> src/main.rs:18:28
   |
18 |     let sub = buffer.slice(500..2000);
   |                            ^^^^^^^^^^ invalid slice range specified here
   |
   = note: GPU slice bounds must reside strictly within the allocated VRAM buffer limits.
   = help: verify that range indices satisfy `start <= end` and `end <= slice.len()`.
```
