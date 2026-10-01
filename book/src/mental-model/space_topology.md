# Spatial Execution & Topology

Every `#[nam]` function declared in Enki takes an immutable reference `&Space` as its first parameter:

```rust
#[nam]
fn my_nam(space: &Space, ...) { ... }
```

The **`Space`** struct represents the spatial execution topology of your workload. It encapsulates global invocation coordinates, local tile (workgroup) geometry, problem boundaries, linear indexing helpers, and graphics projections.

---

## 1. Defining Execution Domains

Enki provides ergonomic constructors to define 1D, 2D, or 3D problem spaces:

```rust
// 1D Domain (e.g. 10 million elements)
let space_1d = Space::gpu_x(10_000_000);

// 2D Domain (e.g. 1920x1080 framebuffer)
let space_2d = Space::gpu_xy(1920, 1080);

// 3D Domain (e.g. 256x256x128 volumetric simulation)
let space_3d = Space::gpu_xyz(256, 256, 128);
```

### Workgroup Tiling Configuration
By default, Enki automatically derives optimal workgroup tile dimensions based on your physical GPU's hardware profile (such as subgroup size and compute unit limits). 

If your algorithm requires specific tile dimensions (for example, when using shared tile memory `GpuTileMem`), you can chain the `.tile(...)` method:

```rust
// 1D: Fixed tile of 256 threads
let space = Space::gpu_x(1_000_000).tile(256);

// 2D: 16x16 tile rectangles
let space = Space::gpu_xy(1920, 1080).tile([16, 16]);

// 3D: 8x8x4 volumetric blocks
let space = Space::gpu_xyz(256, 256, 128).tile([8, 8, 4]);
```

---

## 2. Navigating Coordinates

Inside a `#[nam]`, `Space` provides structured coordinates describing where the current thread is executing

### Global Coordinates
- **`space.x`, `space.y`, `space.z`:** Global invocation coordinates along each axis.
- **`space.pos_xy()` / `space.pos_xyz()`:** Returns tuple pairs or triplets of global coordinates.
- **`space.index()`:** Computes the standard 1D linear flat-memory index:
  ```
  index = x + y * size_x + z * size_x * size_y
  ```



### Local Tile Coordinates
- **`space.cell_x`, `space.cell_y`, `space.cell_z`:** The relative cell coordinate of the thread *inside* its current tile.
- **`space.tile_x`, `space.tile_y`, `space.tile_z`:** The index of the active tile within the global tile grid.
- **`space.cell_index()`:** Computes the 1D linear index of the thread inside its tile scratchpad.

---

## 3. Boundary & Coordination Helpers

When grid dimensions are not exact multiples of workgroup tile sizes, trailing threads may be dispatched beyond the logical problem size. `Space` provides boundary-guarding helpers:

```rust
#[nam]
fn guarded_kernel(space: &Space, pixel: &mut u32) {
    // Exit if the thread falls outside the logical image boundaries
    if !space.in_bounds_xy() {
        return;
    }

    *pixel = 0xFF00FF00;
}
```

- **`space.in_bounds_x()` / `space.in_bounds_xy()` / `space.in_bounds()`:** Returns `true` if the thread is strictly within `(size_x, size_y, size_z)`.
- **`space.is_tile_leader()`:** Returns `true` only for the first cell in a tile (`cell == (0, 0, 0)`). Useful for intra-tile reductions when writing a single aggregated result back to global memory after `space.sync()`.
- **`space.is_global_leader()`:** Returns `true` strictly for the global origin (`pos == (0, 0, 0)`).

---

## 4. Graphics & Color Projections

For raytracing, procedural textures, and presentation pipelines, `Space` provides pre-calculated normalized coordinates:

- **`space.uv()`:** Computes normalized UV coordinates in `[0.0, 1.0]` with standard half-pixel centering (`+0.5`):
  ```text
  u = (x + 0.5) / size_x
  v = (y + 0.5) / size_y
  ```
- **`space.ndc()`:** Computes Normalized Device Coordinates in `[-1.0, 1.0]` for ray direction generation.
- **`space.aspect_ratio()`:** Computes the aspect ratio (`size_x / size_y`).



### Direct Color Packing
To write pixels directly to presentation buffers without third-party color conversion routines, `Space` includes direct color packing methods:

```rust
// Encodes RGB floats (0.0..1.0) into packed 32-bit integer (0xAARRGGBB)
*pixel = space.set_rgb_color(r, g, b);

// Encodes RGBA with custom alpha
*pixel = space.set_rgba_color(r, g, b, a);
```

---

## 5. The CPU Counterpart

When testing or debugging kernels on the CPU, you construct a single-point `Space` instance representing the active thread:

```rust
for y in 0..HEIGHT {
    for x in 0..WIDTH {
        // Construct a 2D CPU execution point
        let space_cpu = Space::cpu_xy(x, y, WIDTH, HEIGHT);
        
        my_nam(&space_cpu, &mut host_buffer[x + y * WIDTH]);
    }
}
```

This ensures that all coordinate methods (`space.x`, `space.uv()`, `space.index()`) return identical values regardless of whether execution occurs on physical GPU hardware or on host CPU threads.
