# By-Value Uniforms (`GpuParam`)

Compute kernels frequently require configuration parameters, transform matrices, time deltas, and camera settings.

In traditional GPU APIs, passing these variables requires writing uniform buffer objects (UBOs) or push constant structures with strict hardware alignment padding rules (`std140` / `std430`).

Enki simplifies this through **`GpuParam<T>`** and the engine's linear **`ParamArena`**.

---

## 1. What is `GpuParam<T>`?

`GpuParam<T>` is a transparent wrapper type that marks an argument as a **by-value uniform**:

```rust
use enki::*;

let dt = GpuParam::new(0.016f32);
```

### Arbitrary Struct Support
`GpuParam<T>` is **not** restricted to primitive numbers (`f32`, `u32`). It supports **any arbitrary Rust composite struct**:

```rust
use glam::{Mat4, Vec3};

#[derive(Clone, Copy, Debug)]
pub struct CameraSettings {
    pub view_projection: Mat4,
    pub eye_position: Vec3,
    pub exposure: f32,
    pub max_bounces: u32,
}
```

---

## 2. The Host-to-Device Contract

Notice the intentional symmetry between how the argument is passed on the host versus how it is declared inside the kernel:

### Inside the `#[nam]` Function
Inside the kernel signature, you declare the parameter **directly by value as `T`**:

```rust
#[nam]
fn raymarch_nam(
    space: &Space,
    pixel: &mut u32,
    camera: CameraSettings, // Received directly by value!
    delta_time: f32,        // Received directly by value!
) {
    let ray_dir = camera.eye_position;
    // ...
}
```

### On the Host Dispatch Side
When calling `.run()` or `.run_unchecked()`, wrap the host instances in `GpuParam::new(...)`:

```rust
let cam = CameraSettings { /* ... */ };
let dt = 0.016f32;

raymarch_nam.run(
    &space,
    &mut screen_buffer,
    GpuParam::new(cam),
    GpuParam::new(dt),
);
```

### On Host CPU Invocations
When calling the kernel directly on host CPU threads for unit testing, pass the instances directly without `GpuParam`:

```rust
// In CPU tests, pass by value directly
raymarch_nam(&space_cpu, &mut cpu_pixel, cam, dt);
```

---

## 3. Under the Hood: The `ParamArena`

`GpuParam<T>` does not allocate a dedicated `VkBuffer` for each uniform. Dedicated GPU allocations introduce heavy PCIe allocation overhead.

Instead, the runtime maintains a high-speed pre-allocated **`ParamArena`**:

```text
Host Memory                         GPU ParamArena (Linear Device Memory)
┌────────────────────────┐          ┌──────────────────────────────────────────────┐
│ cam: CameraSettings    │ ──Upload─► [Offset 0]   CameraSettings (72 bytes + pad) │
│ dt:  0.016f32          │          │ [Offset 80]  delta_time (4 bytes + pad)     │
└────────────────────────┘          └──────────────────────────────────────────────┘
                                                           ▲
                                                           │
                                   PushConstant[0..8] (BDA Base Address)
```

1. **Sequential Packing:** Prior to dispatch, the runtime packs the raw byte payloads of all by-value arguments sequentially into the `ParamArena`.
2. **8-Byte Alignment:** Each uniform entry is automatically aligned to 8-byte boundaries:
   `arena_size = (byte_size + 7) & !7`

3. **Register Pointer:** The 64-bit BDA pointer to the arena base is passed into the first 8 bytes of the GPU push constant register. Invocations load uniforms directly from registers.

---

## 4. Hardware Limits & Configuration (`error[E3010]`)

The default capacity of the `ParamArena` is typically 2 MB. If your dispatch passes large configurations that exceed the available arena capacity, execution halts before recording:

```text
error[E3010]: nam parameters payload exceeds ParamArena capacity
  = note: the total byte footprint of by-value parameters (`GpuParam`) passed to
          this nam exceeds the pre-allocated capacity of the ParamArena.
  = note: configured arena size: x.xx MB
  = note: device maximum allocation capacity: x.xx MB (allocation clamped to
          prevent VRAM allocation fault)
  = help: increase the arena size using `Enki::builder().param_arena_size(...)`
          or pass large arrays by reference (`&GpuVec` or `&[T]`) instead of by
          value.
```

### Customizing Arena Capacity
To customize the capacity allocated for uniform payloads, configure the engine builder during initialization:

```rust
let enki = Enki::builder()
    .param_arena_size(8 * 1024 * 1024) // Allocate 8 MB for uniforms
    .init();
```
