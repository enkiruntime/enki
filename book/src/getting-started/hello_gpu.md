# Your First Nam & Dual Debugging

In Enki, GPU compute kernels are called **Nams** (derived from the Sumerian concept of *nam*—physical decrees or governing principles).

A `#[nam]` is declared as a standard Rust function. It can be compiled and dispatched across parallel GPU execution grids, or invoked on host CPU threads for unit testing and verification.

---

## 1. Writing the Complete Example

Open `src/main.rs` in your project and replace its contents with the following:

```rust
use enki::*;
use glam::Vec2;

const COUNT: usize = 5;

// 1. Declare the compute kernel with #[nam]
#[nam]
fn scale_vectors(_space: &Space, input: &Vec2, output: &mut Vec2, factor: f32) {
    *output = *input * factor;
}

fn main() {
    // 2. Initialize the headless GPU runtime
    let enki = Enki::init();

    // 3. Allocate physical data directly in GPU VRAM
    let in_gpu = gpu_vec![
        Vec2::new(1.0, 2.0),
        Vec2::new(3.0, 4.0),
        Vec2::new(5.0, 6.0),
        Vec2::new(7.0, 8.0),
        Vec2::new(9.0, 10.0),
    ];
    let mut out_gpu = gpu_vec![Vec2::ZERO; COUNT];

    let factor = 2.5f32;

    // 4. Record and submit the GPU execution flow
    enki.flow(|_| {
        scale_vectors.run(
            &Space::gpu_x(COUNT),
            &in_gpu,
            &mut out_gpu,
            GpuParam::new(factor),
        );
    });

    // 5. Dual CPU Execution: Run the exact same function on CPU threads
    let in_cpu = in_gpu.to_vec();
    let mut out_cpu = vec![Vec2::ZERO; COUNT];

    for i in 0..COUNT {
        scale_vectors(&Space::cpu_x(i, COUNT), &in_cpu[i], &mut out_cpu[i], factor);
    }

    // 6. Verify bit-for-bit equivalence
    assert_eq!(&out_cpu[..], &out_gpu.to_vec()[..]);

    println!("GPU Results: {:?}", out_gpu.to_vec());
    println!("Execution verified: CPU and GPU outputs match identically!");
}
```

Run the application:

```bash
cargo run
```

---

## 2. Anatomy of the Dispatch

Let us examine each component of this program:

### The `#[nam]` Signature
```rust
#[nam]
fn scale_vectors(_space: &Space, input: &Vec2, output: &mut Vec2, factor: f32)
```
- **The Execution Context (`&Space`):** The first argument to every nam must be `&Space`. It provides execution metadata such as invocation coordinates (`space.x`), spatial grid bounds, and tile indices.
- **The SPMD Per-Cell Principle:** Notice that `input` is declared as `&Vec2` and `output` as `&mut Vec2`. During execution, Enki maps each thread in the grid exclusively to a single element in the vector.
- **By-Value Parameters:** `factor: f32` is a scalar uniform. Values passed by value are packed into the engine's internal uniform arena (`ParamArena`).

### Allocating GPU Memory (`GpuVec`)
```rust
let in_gpu = gpu_vec![Vec2::new(1.0, 2.0), ...];
let mut out_gpu = gpu_vec![Vec2::ZERO; COUNT];
```
`GpuVec<T>` allocates an owned contiguous buffer in GPU VRAM referenced via a 64-bit Buffer Device Address (BDA). The `gpu_vec!` macro mirrors the ergonomics of `std::vec!`.

### The Active Flow (`enki.flow`)
```rust
enki.flow(|flow| {
    scale_vectors.run(
        &Space::gpu_x(COUNT),
        &in_gpu,
        &mut out_gpu,
        GpuParam::new(factor),
    );
});
```
- A **`flow`** records a sequence of compute and transfer operations into a command buffer synchronized via timeline semaphores.
- **`Space::gpu_x(COUNT)`** defines a 1D execution grid of `COUNT` threads.
- **`GpuParam::new(factor)`**: To satisfy the host-to-device contract, any uniform passed by value must be wrapped in `GpuParam::new(...)` on the host side.

---

## 3. The Dual CPU/GPU Debugging Workflow

Because a `#[nam]` function is a standard Rust function, it can be executed directly on CPU threads:

```rust
for i in 0..COUNT {
    scale_vectors(&Space::cpu_x(i, COUNT), &in_cpu[i], &mut out_cpu[i], factor);
}
```

Or executed in parallel using multi-threaded iterators like `rayon`:

```rust
use rayon::prelude::*;

out_cpu.par_iter_mut().enumerate().for_each(|(i, out)| {
    scale_vectors(&Space::cpu_x(i, COUNT), &in_cpu[i], out, factor);
});
```

This dual-execution model allows developers to:
1. Write standard Rust `#[test]` unit tests that execute in headless CI environments without requiring physical GPU hardware.
2. Step through kernel logic using conventional CPU debuggers (`gdb`, `lldb`).

---

## 4. The Boundaries of CPU Equivalence (An Active Research Area)

While bit-for-bit equivalence between CPU and GPU execution is achieved for element-wise operations, it is critical to understand where CPU simulation currently diverges from silicon execution.

### Where Equivalence Holds (The Deterministic Safe Zone)
Equivalence between host CPU iteration and GPU dispatches is guaranteed for **independent, element-wise algorithms**:
- Per-cell data transformations (`&T` to `&mut T`).
- Pure mathematical evaluations (matrix math, color conversions, ray generation).
- Algorithms with strictly isolated, unshared memory access per invocation.

### Where Divergence Occurs (Hardware-Coupled Features)
Divergence arises when algorithms utilize specialized graphics silicon hardware features:

1. **On-Chip Shared Memory (`GpuTileMem`) & Barriers (`Space::sync()`):**
   On GPU silicon, `GpuTileMem` allocates high-speed on-chip SRAM shared across cooperative threads in a workgroup. Threads execute in lockstep and synchronize at physical memory barriers (`Space::sync()`).
   
   A standard sequential CPU loop (`for i in 0..N`) does not emulate cooperative thread pausing. When iteration `0` encounters `Space::sync()`, it cannot suspend its state to wait for iteration `100` to reach the same barrier. 

2. **Concurrent Atomics (`GpuAtomic`):**
   When hundreds of GPU threads perform concurrent atomic operations (such as `.fetch_add()`), the resolution order depends on non-deterministic hardware warp scheduling. A sequential CPU loop processes operations in a rigid linear order.

3. **Floating-Point Rounding Modes:**
   GPUs commonly execute fused multiply-add (FMA) instructions and fast-math reciprocal approximations that may yield slight least-significant-bit discrepancies compared to x86 CPU FP32 pipelines.

> **Engineering Note: An Active Research Track**  
> Simulating cooperative workgroup memory and barrier synchronization accurately on the CPU requires complex compiler transformations (such as continuation-passing loop splitting or coroutine fiber scheduling). 
> 
> Developing an automated, zero-overhead CPU workgroup simulation harness that accurately mirrors `GpuTileMem` and `Space::sync()` across host threads is an **active area of ongoing research** within the Enki project. Currently, bit-for-bit testing is intended primarily for SPMD safe-mode workloads.
