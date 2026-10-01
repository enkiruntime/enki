# Introduction

**Enki** is a heterogeneous compute platform for Rust. It enables writing algorithms in standard Rust syntax and executing them natively across CPU cores or compiling them just-in-time (JIT) onto GPU hardware.

```rust
use enki::*;

#[nam]
fn transform(space: &Space, in_val: &f32, out_val: &mut f32, scale: f32) {
    *out_val = *in_val * scale;
}
```

The goal of the project is to explore how far a modern systems language can bridge the gap between host logic and accelerator compute without introducing foreign languages, complex binding tables, or abandoning Rust's type-safety invariants.

---

## Motivation and Context

General-purpose GPU computing (GPGPU) has historically developed around dedicated ecosystems:

- **Specialized Platforms (CUDA, OpenCL):** While powerful, these frameworks require writing kernels in vendor-specific dialects (CUDA C++) or older C99 subsets (OpenCL), introducing language barriers and separate compilation pipelines. Newer initiatives like **Mojo** approach this by designing an entirely new language for accelerator programming.
- **Graphics Shading Languages (GLSL, HLSL, WGSL):** These languages were architected primarily around the graphics rendering pipeline (rasterizers, texture sampling, fragment processing). While compute shaders exist in these languages, they operate outside the host language's type system and rely on manual descriptor bindings.

Enki does not seek to replace or compete with hardware-specialized frameworks like CUDA on their own terms. Instead, it investigates a specific systems-programming question:

> **Can standard Rust serve as both the host language and the accelerator language, without introducing a separate shading language, without descriptor set management, and with runtime invariants that preserve Rust's reference semantics?**

---

## Architectural Foundations

The platform is structured around three main principles:

- **Unified Language Surface:** Functions marked with `#[nam]` are standard Rust functions. They can be invoked sequentially or in parallel on CPU threads (e.g., via `rayon`) for testing and debugging, or dispatched across an execution grid on GPU silicon.
- **Physical Memory Addressing:** Enki targets **Vulkan 1.3** and uses 64-bit Buffer Device Addresses (BDA). Buffers are referenced directly through GPU virtual addresses, completely bypassing traditional descriptor set management.
- **Dynamic Invariant Checking:** The runtime includes a verification component (`BorrowEngine`) that checks dispatch arguments before command buffer submission to detect common concurrency bugs, such as overlapping mutable slice access and container size mismatches.

---

## Project Status and Engineering Scope

Enki is currently an **alpha-stage project (`v0.1`)**. It serves as an active exploration into heterogeneous systems design. When evaluating the platform, the following engineering boundaries should be noted:

### Invariant Checking vs. Formal Verification
Enki implements dynamic invariant checking at dispatch time. For example, it verifies that sub-slices passed to a kernel do not overlap in memory, and that buffer allocations are sufficiently large for the requested dispatch domain.

These checks act as a **pragmatic runtime guardrail**. They do not constitute a formal, mathematically verified soundness proof for arbitrary parallel access patterns. When dispatches are executed in unchecked mode (`run_unchecked`), synchronization correctness remains the responsibility of the developer.

### API Stability
The runtime interfaces, internal compiler passes, and data structures are subject to refinement as the system matures. The platform is designed for experimentation, evaluation, and community feedback.

---

## System Requirements

Running Enki requires hardware and driver support for modern Vulkan features:

- **Rust:** Stable toolchain (`1.80` or newer).
- **Vulkan Driver:** Support for Vulkan 1.3 or higher, with the following capabilities:
  - 64-bit Buffer Device Addresses (`VK_KHR_buffer_device_address`)
  - Timeline Semaphores (`VK_KHR_timeline_semaphore`)
  - Synchronization2 (`VK_KHR_synchronization2`)
  - 64-bit Shader Integers (`shaderInt64`)
- **Supported Platforms:** Linux and Windows.

---

## Organization of This Book

- **[Getting Started](getting-started/README.md):** Environment setup, compilation prerequisites, and writing your first compute dispatch.
- **[The Mental Model](mental-model/README.md):** How Rust types and references map to physical GPU memory structures.
- **[Memory & Resource System](resources/README.md):** Details on `GpuVec`, buffer slicing, uniforms (`GpuParam`), and hardware atomics.
- **[Compiler Invariants](compiler-invariants/README.md):** The constraints enforced by the JIT compiler backend when lowering Rust to GPU bytecode.
- **[The GPU BorrowEngine](borrow-engine/README.md):** The mechanics of dynamic contract matching and spatial safety checks.
- **[Graphics & Under the Hood](graphics-and-engine/README.md):** Interactive window presentation and the graph-based barrier generation engine.
- **[Diagnostics Reference](diagnostics-index/README.md):** Catalog of diagnostic error codes and messages.
