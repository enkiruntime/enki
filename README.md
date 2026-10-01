<div align="center">

# Enki

**Pure-Rust Heterogeneous GPU Compute Platform with Just-In-Time Compilation**
  
Write standard, idiomatic Rust functions and execute them natively across CPU cores or compile them just-in-time directly onto GPU silicon.

*No Nightly toolchains required. No foreign shading languages. No descriptor set juggling.*

[![Crates.io](https://img.shields.io/crates/v/enki-gpu.svg)](https://crates.io/crates/enki-gpu)
[![Documentation](https://img.shields.io/badge/docs-enki_book-blue.svg)](https://enkiruntime.github.io/enki/)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

</div>

---

## Interactive Showcase: Real-Time 3D Raymarching

Enki unifies host and accelerator execution. The showcase below demonstrates a real-time procedural 3D raymarching with smooth lighting, ambient occlusion, and mouse-driven orbital camera controls—compiled just-in-time from a single Rust function running on a potato GPU:

<div align="center">

![Enki 3D Raymarching Showcase](./media/enki_showcase.gif)

*Pressing `SPACE` dynamically switches execution between GPU silicon (Enki) and multi-threaded CPU cores (Rayon) in real time.*

</div>

### Run the Demo:

```bash
git clone https://github.com/enkiruntime/enki_sdf.git
cd enki_sdf
cargo run
```

---

## Quick Start

Add Enki and `glam` to your project:

```bash
cargo add enki-gpu glam
```

Replace `src/main.rs` with the following:

```rust
use enki::*;
use glam::Vec2;

const COUNT: usize = 5;

// Declare the compute kernel with #[nam]
#[nam]
fn scale_vectors(_space: &Space, input: &Vec2, output: &mut Vec2, factor: f32) {
    *output = *input * factor;
}

fn main() {
    // Initialize the headless GPU runtime
    let enki = Enki::init();

    // Allocate physical data directly in GPU VRAM
    let in_gpu = gpu_vec![
        Vec2::new(1.0, 2.0),
        Vec2::new(3.0, 4.0),
        Vec2::new(5.0, 6.0),
        Vec2::new(7.0, 8.0),
        Vec2::new(9.0, 10.0),
    ];
    let mut out_gpu = gpu_vec![Vec2::ZERO; COUNT];

    let factor = 2.5f32;

    // Record and dispatch directly to GPU silicon
    enki.flow(|_| {
        scale_vectors.run(
            &Space::gpu_x(COUNT),
            &in_gpu,
            &mut out_gpu,
            GpuParam::new(factor),
        );
    });

    // Dual Execution: Run the exact same function on CPU native rust
    let in_cpu = vec![
        Vec2::new(1.0, 2.0),
        Vec2::new(3.0, 4.0),
        Vec2::new(5.0, 6.0),
        Vec2::new(7.0, 8.0),
        Vec2::new(9.0, 10.0),
    ];
    let mut out_cpu = vec![Vec2::ZERO; COUNT];

    for i in 0..COUNT {
        scale_vectors(&Space::cpu_x(i, COUNT), &in_cpu[i], &mut out_cpu[i], factor);
    }

    // Verify bit-for-bit equivalence
    assert_eq!(&out_cpu[..], &out_gpu.to_vec()[..]);
    println!("GPU Results: {:?}", out_gpu.to_vec());
    println!("Execution verified: CPU and GPU outputs match identically!");
}
```

Run the application:

```bash
cargo run
```

*Note: On your first build, Enki will prompt to automatically configure `--emit=llvm-bc` and optimization profiles in `.cargo/config.toml`. Alternatively, you can run non-intrusively using the official CLI runner via `cargo install cargo-enki && cargo enki run`.*

---

## Architectural Highlights

- **Runs on Stable Rust:** Operates on standard stable Rust (`1.80+`). No nightly compiler forks, custom toolchains, or experimental compiler plugins.
- **Physical Memory Addressing:** Built natively on **Vulkan 1.3** and 64-bit Buffer Device Addresses (BDA). Eliminates descriptor pools, descriptor sets, and binding tables.
- **Two-Tier Borrow Checking:** Combines `rustc`'s compile-time borrow checker on the host with a runtime `BorrowEngine` that intercepts spatial slice collisions, domain bound deficits, and temporal presentation hazards.
- **Automated Synchronization (TTRD):** An internal Transitive Reduction Dependency Solver constructs a directed hazard graph and derives the mathematically minimal set of Vulkan pipeline barriers (`Synchronization2`) automatically.
- **Dual CPU/GPU Verification:** Functions marked with `#[nam]` are standard Rust functions. They can be tested natively on host CPU threads using `rayon` and standard `#[test]` assertions without requiring physical GPU silicon in CI/CD pipelines.

---

## Project Status & Pragmatic Safety

Enki is currently an **alpha-stage project (`v0.1`)**. It represents an active systems research effort into unified language execution:

- **Dynamic Invariant Checking:** The `BorrowEngine` operates as a **compiler-grade pragmatic safety net** at the dispatch boundary. It detects concrete spatial and temporal data race hazards before hardware queue submission. It does **not** claim to provide formal mathematical soundness proofs for arbitrary parallel access patterns.
- **Evolving Interfaces:** Runtime APIs, compiler lowering passes, and internal data structures are subject to refinement as the system matures.

---

## The Enki Book

For in-depth architectural breakdowns, memory layout analysis, and advanced graphics pipelines, read the official documentation:

--> **[Read The Enki Book](https://enkiruntime.github.io/enki/)**

The book covers:
- **The Mental Model:** How Rust references map to physical 64-bit GPU virtual addresses.
- **Memory & Resource System:** Deep dives into `GpuVec`, zero-cost sub-slicing (`Slice`), by-value uniform packing (`GpuParam`), and on-chip scratchpad memory (`GpuTileMem`).
- **Compiler Invariants:** Understanding hardware constraints on GPU silicon (divergence, heap allocations, panics).
- **The GPU BorrowEngine:** Mechanics of spatial disjointness and temporal presentation lifecycles.
- **Graphics & Presentation:** Building real-time interactive display applications with GLFW and swapchains.

---

## System Requirements

- **Rust Toolchain:** Stable Rust `1.80` or newer.
- **Graphics Driver:** Official GPU driver with Vulkan 1.3 support, including:
  - 64-bit Buffer Device Addresses (`VK_KHR_buffer_device_address`)
  - Timeline Semaphores (`VK_KHR_timeline_semaphore`)
  - Synchronization2 (`VK_KHR_synchronization2`)
  - 64-bit Shader Integers (`shaderInt64`)
- **Supported Operating Systems:** Linux (X11 / Wayland), Windows (10 / 11).

---

## Licensing

Enki is structured with an open-core architecture:

* **Enki Framework (`enki-gpu`, `anu`, `apsu`, `utu`, `enki_macros`):** 
  Fully open-source under [MIT License](LICENSE-MIT) or [Apache License 2.0](LICENSE-APACHE).
* **Parsu GPU Compiler Backend (Pre-compiled Binary):** 
  Distributed under the [Parsu License & Commercial Notice](PARSU_LICENSE.md). 
  It is **free forever** for developers, researchers, and open-source use. 
  Commercial production usage will require commercial licensing in future releases.

---
