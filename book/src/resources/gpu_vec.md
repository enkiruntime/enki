# Owned VRAM with `GpuVec`

`GpuVec<T>` represents a contiguous, growable array allocated directly in GPU VRAM. It serves as the primary owned container for large datasets in Enki.

```rust
use enki::*;

// Allocate directly in GPU device memory
let mut buffer: GpuVec<f32> = gpu_vec![1.0, 2.0, 3.0, 4.0];
```

---

## 1. Allocation and Construction

`GpuVec<T>` mirrors standard `Vec<T>` constructors while allocating memory through the physical device allocator (`apsu`):

- **`GpuVec::new()` / `with_capacity(cap)`:** Allocates a storage buffer with Buffer Device Address (BDA) capability.
- **`GpuVec::from_slice(&[T])`:** Allocates VRAM and performs a synchronous host-to-device upload.
- **`GpuVec::from_elem(value, count)`:** Allocates and initializes `count` elements with `value`.
- **`GpuVec::zeroed(count)`:** Allocates and zeroes out device memory via transfer commands.

> **Zero-Sized Types (ZST):** GPU silicon requires every memory access to have a concrete physical stride. Attempting to instantiate a `GpuVec<()>` or any struct with a byte size of zero will assert at initialization.

---

## 2. Ownership and Deep Cloning

`GpuVec<T>` follows standard Rust move semantics. When you call `.clone()` on a `GpuVec`:

```rust
let v1 = gpu_vec![42.0f32; 1000];
let v2 = v1.clone(); // Physical VRAM-to-VRAM copy
```

Enki allocates an entirely distinct physical buffer with a unique 64-bit BDA and unique engine slot ID, then executes a hardware memory transfer (`vkCmdCopyBuffer`) to copy the contents. Mutating `v2` on the GPU has zero impact on `v1`.

---

## 3. Host Synchronization & Timeline Semaphores

Because GPU execution is asynchronous, `GpuVec<T>` tracks its execution phase relative to the engine's Vulkan timeline semaphores:

```text
              GPU Execution Timeline (Time Progresses ──>)
──[Submitted Batch (Timeline: 42)]──────[GPU Reaches 42 (Idle)]──>
             ▲                                      ▲
             │                                      │
   host.get(i) called here                GPU finishes work
   (CPU automatically blocks)             (Readback completes)
```

When you read data back to the host CPU:

```rust
// Reads a single element
let val: Option<f32> = buffer.get(0);

// Reads all elements back into a standard heap Vec<T>
let host_data: Vec<f32> = buffer.to_vec();
```

Enki automatically queries the underlying timeline semaphore. If the buffer is currently involved in executing GPU dispatches, the CPU thread **automatically waits** until the GPU finishes writing before transferring bytes across the PCIe bus.

---

## 4. Phase Invariants: Recording vs. Execution (`error[E2001]`)

While reading back from an idle or in-flight buffer on the CPU is safe, doing so **inside an active flow recording block is strictly forbidden**:

```rust
enki.flow(|flow| {
    my_nam.run(&space, &mut buffer);

    // ERROR: Illegal host readback during command recording
    let val = buffer.get(0); 
});
```

### Why is this rejected?
Inside `enki.flow`, compute commands are actively being recorded into a Vulkan command buffer—they have not yet been submitted to the GPU queue. 

Attempting to read back `buffer.get(0)` at this moment would require the CPU to wait for work that has not even been submitted, causing a permanent CPU-GPU deadlock. 

To prevent this, the runtime immediately aborts with a diagnostic error:

```text
error[E2001]: illegal host readback during GPU command recording phase
   --> src/main.rs:302:49
    |
302 |                     let cpu_pixels = gpu_pixels.to_vec();
    |                                                 ^^^^^^^^ host readback attempted here
    |
    = note: dispatches inside `enki.flow` are recorded into a command buffer
            and have not been submitted to the GPU; waiting for results here
            causes a permanent CPU-GPU deadlock.
    = note: `to_vec()` was invoked on the CPU while recording GPU commands
            inside `enki.flow`
    = help: move readback operations (`.to_vec()`, `.get()`, or `println!`)
            outside the `enki.flow` closure.
```
