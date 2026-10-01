/// Spatial execution context provided to every invocation of a `#[nam]` function.
///
/// Encapsulates global thread coordinates, local tile topology, domain boundaries,
/// flat memory indexing helpers, and intra-tile synchronization barriers.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
#[repr(C)]
pub struct Space {
    /// Global execution coordinate along the X axis.
    pub x: usize,
    /// Global execution coordinate along the Y axis.
    pub y: usize,
    /// Global execution coordinate along the Z axis.
    pub z: usize,

    /// Local cell coordinate along the X axis inside the current tile.
    pub cell_x: usize,
    /// Local cell coordinate along the Y axis inside the current tile.
    pub cell_y: usize,
    /// Local cell coordinate along the Z axis inside the current tile.
    pub cell_z: usize,

    /// Tile index along the X axis in the global tile grid.
    pub tile_x: usize,
    /// Tile index along the Y axis in the global tile grid.
    pub tile_y: usize,
    /// Tile index along the Z axis in the global tile grid.
    pub tile_z: usize,

    /// Total global problem size along the X axis.
    pub size_x: usize,
    /// Total global problem size along the Y axis.
    pub size_y: usize,
    /// Total global problem size along the Z axis.
    pub size_z: usize,

    /// Local tile capacity dimension along the X axis.
    pub tile_size_x: usize,
    /// Local tile capacity dimension along the Y axis.
    pub tile_size_y: usize,
    /// Local tile capacity dimension along the Z axis.
    pub tile_size_z: usize,
}

impl Space {
    /// Synchronizes all execution threads within the current spatial tile.
    ///
    /// # Diagnostics
    ///
    /// Triggers a compile-error `error[E0009]` if all cells in a tile do not reach `space.sync()` concurrently.
    ///
    /// Does nothing if the `#[nam]` function is called on the CPU.
    #[inline(always)]
    pub fn sync(&self) {
        __enki_sync();
    }

    /// Returns the global coordinate along the X axis.
    #[inline(always)]
    pub const fn pos_x(&self) -> usize {
        self.x
    }

    /// Returns the global (x, y) coordinates as a 2D tuple.
    #[inline(always)]
    pub const fn pos_xy(&self) -> (usize, usize) {
        (self.x, self.y)
    }

    /// Returns the global (x, y, z) coordinates as a 3D tuple.
    #[inline(always)]
    pub const fn pos_xyz(&self) -> (usize, usize, usize) {
        (self.x, self.y, self.z)
    }

    /// Returns the local cell coordinate along the X axis inside the tile.
    #[inline(always)]
    pub const fn cell_x(&self) -> usize {
        self.cell_x
    }

    /// Returns the local cell (cell_x, cell_y) coordinates inside the tile.
    #[inline(always)]
    pub const fn cell_xy(&self) -> (usize, usize) {
        (self.cell_x, self.cell_y)
    }

    /// Returns the local cell (cell_x, cell_y, cell_z) coordinates inside the tile.
    #[inline(always)]
    pub const fn cell_xyz(&self) -> (usize, usize, usize) {
        (self.cell_x, self.cell_y, self.cell_z)
    }

    /// Returns the tile index along the X axis in the global tile grid.
    #[inline(always)]
    pub const fn tile_x(&self) -> usize {
        self.tile_x
    }

    /// Returns the tile index (tile_x, tile_y) in the global tile grid.
    #[inline(always)]
    pub const fn tile_xy(&self) -> (usize, usize) {
        (self.tile_x, self.tile_y)
    }

    /// Returns the tile index (tile_x, tile_y, tile_z) in the global tile grid.
    #[inline(always)]
    pub const fn tile_xyz(&self) -> (usize, usize, usize) {
        (self.tile_x, self.tile_y, self.tile_z)
    }

    /// Returns the global domain size along the X axis.
    #[inline(always)]
    pub const fn size_x(&self) -> usize {
        self.size_x
    }

    /// Returns the global domain (size_x, size_y) as a 2D tuple.
    #[inline(always)]
    pub const fn size_xy(&self) -> (usize, usize) {
        (self.size_x, self.size_y)
    }

    /// Returns the global domain (size_x, size_y, size_z) as a 3D tuple.
    #[inline(always)]
    pub const fn size_xyz(&self) -> (usize, usize, usize) {
        (self.size_x, self.size_y, self.size_z)
    }

    /// Returns the tile dimension along the X axis.
    #[inline(always)]
    pub const fn tile_size_x(&self) -> usize {
        self.tile_size_x
    }

    /// Returns the tile dimensions (tile_size_x, tile_size_y) as a 2D tuple.
    #[inline(always)]
    pub const fn tile_size_xy(&self) -> (usize, usize) {
        (self.tile_size_x, self.tile_size_y)
    }

    /// Returns the tile dimensions (tile_size_x, tile_size_y, tile_size_z) as a 3D tuple.
    #[inline(always)]
    pub const fn tile_size_xyz(&self) -> (usize, usize, usize) {
        (self.tile_size_x, self.tile_size_y, self.tile_size_z)
    }

    /// Returns the number of tiles along the X axis in the global grid.
    #[inline(always)]
    pub fn tile_count_x(&self) -> usize {
        self.size_x.div_ceil(self.tile_size_x.max(1))
    }

    /// Returns the number of tiles along the Y axis in the global grid.
    #[inline(always)]
    pub fn tile_count_y(&self) -> usize {
        self.size_y.div_ceil(self.tile_size_y.max(1))
    }

    /// Returns the number of tiles along the Z axis in the global grid.
    #[inline(always)]
    pub fn tile_count_z(&self) -> usize {
        self.size_z.div_ceil(self.tile_size_z.max(1))
    }

    /// Returns the number of tiles (count_x, count_y) in the global grid.
    #[inline(always)]
    pub fn tile_count_xy(&self) -> (usize, usize) {
        let tx = self.size_x.div_ceil(self.tile_size_x.max(1));
        let ty = self.size_y.div_ceil(self.tile_size_y.max(1));
        (tx, ty)
    }

    /// Returns the number of tiles (count_x, count_y, count_z) in the global grid.
    #[inline(always)]
    pub fn tile_count_xyz(&self) -> (usize, usize, usize) {
        let tx = self.size_x.div_ceil(self.tile_size_x.max(1));
        let ty = self.size_y.div_ceil(self.tile_size_y.max(1));
        let tz = self.size_z.div_ceil(self.tile_size_z.max(1));
        (tx, ty, tz)
    }

    /// Returns the total number of tiles across all dimensions in the grid.
    #[inline(always)]
    pub fn total_tiles(&self) -> usize {
        let (tx, ty, tz) = self.tile_count_xyz();
        tx * ty * tz
    }

    /// Computes the 1D linear flat memory index for the current invocation.
    ///
    /// Evaluates `x + y * size_x + z * size_x * size_y`.
    /// Works symmetrically across 1D, 2D, and 3D execution domains.
    #[inline(always)]
    pub const fn index(&self) -> usize {
        self.x + self.y * self.size_x + self.z * self.size_x * self.size_y
    }

    /// Computes the 1D linear cell index of this invocation inside the current tile.
    ///
    /// Evaluates `cell_x + cell_y * tile_size_x + cell_z * tile_size_x * tile_size_y`.
    #[inline(always)]
    pub const fn cell_index(&self) -> usize {
        self.cell_x
            + self.cell_y * self.tile_size_x
            + self.cell_z * self.tile_size_x * self.tile_size_y
    }

    /// Computes the 1D linear index of the current tile in the global tile grid.
    #[inline(always)]
    pub fn tile_index(&self) -> usize {
        let (tx, ty) = self.tile_count_xy();
        self.tile_x + self.tile_y * tx + self.tile_z * tx * ty
    }

    /// Returns the total execution domain size (`size_x * size_y * size_z`).
    #[inline(always)]
    pub const fn total_cells(&self) -> usize {
        self.size_x * self.size_y * self.size_z
    }

    /// Returns `true` if the current invocation is within 1D bounds (`x < size_x`).
    #[inline(always)]
    pub const fn in_bounds_x(&self) -> bool {
        self.x < self.size_x
    }

    /// Returns `true` if the current invocation is within 2D bounds (`x < size_x && y < size_y`).
    #[inline(always)]
    pub const fn in_bounds_xy(&self) -> bool {
        self.x < self.size_x && self.y < self.size_y
    }

    /// Returns `true` if the current invocation is within 3D bounds (`x < size_x && y < size_y && z < size_z`).
    #[inline(always)]
    pub const fn in_bounds_xyz(&self) -> bool {
        self.x < self.size_x && self.y < self.size_y && self.z < self.size_z
    }

    /// Returns `true` if the current invocation is within all active domain dimensions.
    #[inline(always)]
    pub const fn in_bounds(&self) -> bool {
        self.in_bounds_xyz()
    }

    /// Returns `true` if this invocation is the leader of the current tile (`cell == (0, 0, 0)`).
    ///
    /// Used in intra-tile reductions to perform single-thread writes to global memory after [`Self::sync`].
    #[inline(always)]
    pub const fn is_tile_leader(&self) -> bool {
        self.cell_x == 0 && self.cell_y == 0 && self.cell_z == 0
    }

    /// Returns `true` if this invocation is the global origin of the entire dispatch (`pos == (0, 0, 0)`).
    #[inline(always)]
    pub const fn is_global_leader(&self) -> bool {
        self.x == 0 && self.y == 0 && self.z == 0
    }

    /// Computes 2D normalized UV coordinates in `[0.0, 1.0]` with pixel-center alignment (`+0.5`).
    #[inline(always)]
    pub fn uv(&self) -> (f32, f32) {
        (
            (self.x as f32 + 0.5) / self.size_x as f32,
            (self.y as f32 + 0.5) / self.size_y.max(1) as f32,
        )
    }

    /// Computes 3D normalized volumetric UVW coordinates in `[0.0, 1.0]` with voxel-center alignment.
    #[inline(always)]
    pub fn uvw(&self) -> (f32, f32, f32) {
        (
            (self.x as f32 + 0.5) / self.size_x as f32,
            (self.y as f32 + 0.5) / self.size_y.max(1) as f32,
            (self.z as f32 + 0.5) / self.size_z.max(1) as f32,
        )
    }

    /// Computes Normalized Device Coordinates (NDC) in `[-1.0, 1.0]` for ray generation.
    #[inline(always)]
    pub fn ndc(&self) -> (f32, f32) {
        let (u, v) = self.uv();
        (u * 2.0 - 1.0, 1.0 - v * 2.0)
    }

    /// Computes the aspect ratio of the 2D domain (`size_x / size_y`).
    #[inline(always)]
    pub fn aspect_ratio(&self) -> f32 {
        self.size_x as f32 / self.size_y.max(1) as f32
    }

    /// Encodes normalized floating-point RGBA components into a packed `u32` integer (`0xAABBGGRR` / `0xAARRGGBB`).
    #[inline(always)]
    pub fn set_rgba_color(&self, r: f32, g: f32, b: f32, a: f32) -> u32 {
        let a = (a.clamp(0.0, 1.0) * 255.0) as u32;
        let r = (r.clamp(0.0, 1.0) * 255.0) as u32;
        let g = (g.clamp(0.0, 1.0) * 255.0) as u32;
        let b = (b.clamp(0.0, 1.0) * 255.0) as u32;

        (a << 24) | (r << 16) | (g << 8) | b
    }

    /// Encodes normalized RGB components into a packed `u32` integer with full alpha (`a = 1.0`).
    #[inline(always)]
    pub fn set_rgb_color(&self, r: f32, g: f32, b: f32) -> u32 {
        self.set_rgba_color(r, g, b, 1.0)
    }

    /// Encodes normalized RG components into a packed `u32` integer (`b = 0.0, a = 1.0`).
    #[inline(always)]
    pub fn set_rg_color(&self, r: f32, g: f32) -> u32 {
        self.set_rgba_color(r, g, 0.0, 1.0)
    }

    /// Encodes normalized red component into a packed `u32` integer (`g = 0.0, b = 0.0, a = 1.0`).
    #[inline(always)]
    pub fn set_r_color(&self, r: f32) -> u32 {
        self.set_rgba_color(r, 0.0, 0.0, 1.0)
    }

    /// Encodes normalized green component into a packed `u32` integer (`r = 0.0, b = 0.0, a = 1.0`).
    #[inline(always)]
    pub fn set_g_color(&self, g: f32) -> u32 {
        self.set_rgba_color(0.0, g, 0.0, 1.0)
    }

    /// Encodes normalized blue component into a packed `u32` integer (`r = 0.0, g = 0.0, a = 1.0`).
    #[inline(always)]
    pub fn set_b_color(&self, b: f32) -> u32 {
        self.set_rgba_color(0.0, 0.0, b, 1.0)
    }

    /// Encodes normalized alpha component into a packed `u32` integer with black color.
    #[inline(always)]
    pub fn set_a_color(&self, a: f32) -> u32 {
        self.set_rgba_color(0.0, 0.0, 0.0, a)
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
#[cold]
pub extern "C" fn __enki_sync() {
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
    core::hint::black_box(());
}
