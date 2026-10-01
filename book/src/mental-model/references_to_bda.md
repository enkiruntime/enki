# Rust References to 64-bit BDA

In traditional GPU programming models (Vulkan, DirectX 12, WebGPU), passing memory to a compute shader requires constructing **Descriptor Tables**:

```text
[Host Code] ──> Descriptor Pool ──> Descriptor Set ──> Pipeline Layout ──> Shader Binding
```

This indirection introduces fragile binding indices (`@binding(0)`), descriptor allocation overhead, and a disconnect between host and device type systems.

Enki takes an alternative approach enabled by **Vulkan 1.3**: physical **Buffer Device Addresses (BDA)**.

---

## 1. What is Buffer Device Addressing (BDA)?

On modern GPU hardware, VRAM allocations reside in a unified 64-bit virtual address space. With the `VK_KHR_buffer_device_address` extension:

- Every buffer allocated on the GPU possesses a native 64-bit hardware address (`u64`).
- Compute shaders can dereference these addresses directly through physical machine registers, exactly like pointers in standard CPU architectures.

By leveraging BDA, Enki bypasses descriptor sets entirely. Buffers are treated as direct memory pointers rather than bound resources.

---

## 2. The Ingress Architecture: 32-Byte Push Constants

When you dispatch a `#[nam]` inside an `enki.flow`, how do these 64-bit addresses reach the GPU threads?

Instead of binding descriptor sets, Enki configures a single, high-speed **32-byte Push Constant block** that is passed directly to graphics hardware registers:

```text
                          32-Byte Push Constant Register
┌───────────────────┬──────────────┬──────────────┬──────────────┬─────────┬───────────────────┐
│ Bytes 0..8        │ Bytes 8..12  │ Bytes 12..16 │ Bytes 16..20 │ 20..24  │ Bytes 24..32      │
│ ParamArena BDA    │ Grid Size X  │ Grid Size Y  │ Grid Size Z  │ Padding │ Stack Buffer BDA  │
└─────────┬─────────┴──────────────┴──────────────┴──────────────┴─────────┴───────────────────┘
          │
          ▼
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ GPU ParamArena (Contiguous Uniform Memory)                                                   │
│                                                                                              │
│  [0..8]   Arg 0: Base BDA of GpuVec A (0x7F00_1200)                                          │
│  [8..24]  Arg 1: Slice BDA + Element Count (0x7F00_3400, 1024)                               │
│  [24..28] Arg 2: By-Value Uniform Float (e.g. dt: 0.016f32)                                  │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

1. **`ParamArena BDA` (Bytes 0..8):** Points to a linear uniform buffer allocated on the device. All function arguments—whether they are scalar uniforms or buffer device addresses—are packed sequentially into this arena.
2. **`Grid Dimensions` (Bytes 8..20):** Conveys the physical $(X, Y, Z)$ bounds of the current dispatch domain as three 32-bit integers.
3. **`Stack BDA` (Bytes 24..32):** Points to dedicated VRAM stack space if the kernel requires dynamic stack frames (aligned after 4-byte padding).

This means **zero descriptor set allocations and zero pipeline layout rebinding** occur across dispatches.

---

## 3. How Rust Types Map to Silicon Memory

When you declare arguments in a `#[nam]` function, Enki maps them into physical silicon structures according to standard Rust semantics:

| Rust Parameter in `#[nam]` | Ingress Representation | Silicon Hardware Execution |
| :--- | :--- | :--- |
| **`value: T`** (by value) | Payload bytes in `ParamArena` | Loaded into uniform registers (`PushConstant`/Arena). |
| **`cell: &T` / `&mut T`** | 64-bit Buffer Base Address | Offset per-thread: `base_bda + (thread_id * stride)`. |
| **`slice: &[T]` / `&mut [T]`** | 16-byte Fat Pointer `(bda, count)` | Indexed access across full container bounds. |
| **`atomic: &AtomicU32`** | 64-bit Device Address | Hardware atomic engine instructions. |
| **`tile: &mut [T; N]`** | Zero footprint (0 bytes) | High-speed on-chip SRAM (`Workgroup` LDS memory). |

In the next chapter, we explore how this direct addressing model forms the foundation of **The SPMD Per-Cell Principle**.
