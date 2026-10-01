# Hardware Atomics (`GpuAtomic` & `GpuAtomicVec`)

Parallel GPU algorithms frequently require multi-threaded coordination, such as global compaction counters, work-queue distribution, and category binning.

In traditional shading languages, developers must rely on specialized shader intrinsics (e.g. `atomicAdd()`, `atomicCompSwap()`).

Enki maps GPU hardware atomic instructions directly to Rust's **standard library atomic types** (`core::sync::atomic`).

---

## 1. The `GpuAtomicTarget` Trait

Enki defines the `GpuAtomicTarget` trait for integer primitives that have native hardware atomic support on graphics silicon:

| Host Scalar Type | `#[nam]` Kernel Target Type | Hardware Instruction Mapping |
| :--- | :--- | :--- |
| **`u32`** | **`&core::sync::atomic::AtomicU32`** | Native 32-bit hardware atomics (`OpAtomicIAdd`, etc.) |
| **`i32`** | **`&core::sync::atomic::AtomicI32`** | Native signed 32-bit hardware atomics |
| **`u64`** | **`&core::sync::atomic::AtomicU64`** | 64-bit integer atomics (requires `shaderInt64`) |
| **`i64`** | **`&core::sync::atomic::AtomicI64`** | 64-bit signed integer atomics |
| **`usize`** | **`&core::sync::atomic::AtomicUsize`** | Mapped to native 64-bit device pointer width |

Inside a `#[nam]`, you interact with atomics using standard Rust atomic methods: `.fetch_add()`, `.fetch_sub()`, `.fetch_min()`, `.fetch_max()`, `.load()`, and `.store()`.

---

## 2. Scalar Atomics (`GpuAtomic<T>`)

`GpuAtomic<T>` represents a single, isolated hardware atomic variable allocated in GPU VRAM:

```rust
use enki::*;
use core::sync::atomic::{AtomicU32, Ordering};

#[nam]
fn count_active_particles(space: &Space, speed: &f32, counter: &AtomicU32) {
    if !space.in_bounds_x() {
        return;
    }

    if *speed > 10.0 {
        // Direct hardware atomic addition on silicon
        counter.fetch_add(1, Ordering::Relaxed);
    }
}

fn main() {
    let enki = Enki::init();

    // Allocate an atomic counter in VRAM initialized to 0
    let counter = GpuAtomic::new(0u32);
    let speeds = gpu_vec![5.0f32, 12.0, 3.0, 15.0, 8.0];

    enki.flow(|_| {
        count_active_particles.run(&Space::gpu_x(5), &speeds, &counter);
    });

    // Read back to CPU (automatically waits on timeline)
    let total_active = counter.get();
    println!("Active particles (>10.0): {}", total_active);
    assert_eq!(total_active, 2);
}
```

---

## 3. Atomic Arrays (`GpuAtomicVec<T>`)

When algorithms require a shared array of atomic variables (such as category classification or spatial hash grids), use `GpuAtomicVec<T>`.

In this example, 1,000 parallel threads classify sensor temperature readings into **4 distinct alert buckets**:

```rust
use enki::*;
use core::sync::atomic::{AtomicU32, Ordering};

const NUM_CATEGORIES: usize = 4;

#[nam]
fn classify_sensors(space: &Space, temp: &f32, buckets: &[AtomicU32]) {
    if !space.in_bounds_x() {
        return;
    }

    // Determine category: 0 = Normal, 1 = Warning, 2 = High, 3 = Critical
    let bucket_idx = if *temp < 25.0 {
        0
    } else if *temp < 50.0 {
        1
    } else if *temp < 75.0 {
        2
    } else {
        3
    };

    // Concurrent multi-threaded atomic increment into the target bucket
    buckets[bucket_idx].fetch_add(1, Ordering::Relaxed);
}

fn main() {
    let enki = Enki::init();

    // 1. Allocate an array of 4 atomic counters in VRAM
    let buckets = GpuAtomicVec::<u32>::new(&[0u32; NUM_CATEGORIES]);

    // 2. Prepare 1,000 sensor readings
    let mut host_readings = Vec::with_capacity(1000);
    for i in 0..1000 {
        host_readings.push((i as f32 * 0.1) % 100.0);
    }
    let sensor_data = GpuVec::from_slice(&host_readings);

    // 3. Dispatch across 1,000 threads in Safe Mode
    enki.flow(|_| {
        classify_sensors.run(&Space::gpu_x(1000), &sensor_data, &buckets);
    });

    // 4. Read back the 4 buckets to the host
    let results = buckets.to_vec().unwrap();
    println!("Sensor Classification Results:");
    println!("  Normal   (<25.0C):  {}", results[0]);
    println!("  Warning  (25..50C): {}", results[1]);
    println!("  High     (50..75C): {}", results[2]);
    println!("  Critical (>=75.0C): {}", results[3]);

    // Total classified items must equal 1,000
    assert_eq!(results.iter().sum::<u32>(), 1000);
}
```

---

## 4. Parallel Safety in Safe Mode

In standard dispatches, passing a mutable slice (`SliceMut`) across parallel threads triggers restrictions because unconstrained concurrent writes to arbitrary indices introduce data races.

With `GpuAtomic<T>` and `GpuAtomicVec<T>`, the situation is fundamentally different:
- Hardware atomic operations are **atomic by definition at the silicon transistor level**.
- Even if hundreds of threads attempt to execute `.fetch_add(1)` on the exact same bucket simultaneously, the GPU memory controller serializes the memory requests cleanly.

For this reason, Enki's `BorrowEngine` explicitly **permits concurrent shared atomic references in Safe Mode (`.run()`)**.
