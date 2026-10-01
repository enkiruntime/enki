use super::core::Space;

impl Space {
    /// Creates a 1D CPU execution point for coordinate `x` within domain size `size_x`.
    ///
    /// Can be chained with [`.tile()`](Self::tile) to configure local tile coordinates.
    #[inline(always)]
    pub fn cpu_x(x: usize, size_x: usize) -> Self {
        Self {
            x,
            y: 0,
            z: 0,
            cell_x: 0,
            cell_y: 0,
            cell_z: 0,
            tile_x: x,
            tile_y: 0,
            tile_z: 0,
            size_x: size_x.max(1),
            size_y: 1,
            size_z: 1,
            tile_size_x: 1,
            tile_size_y: 1,
            tile_size_z: 1,
        }
    }

    /// Creates a 2D CPU execution point for coordinate `(x, y)` within domain size `(size_x, size_y)`.
    ///
    /// Can be chained with [`.tile()`](Self::tile) to configure local tile coordinates.
    #[inline(always)]
    pub fn cpu_xy(x: usize, y: usize, size_x: usize, size_y: usize) -> Self {
        Self {
            x,
            y,
            z: 0,
            cell_x: 0,
            cell_y: 0,
            cell_z: 0,
            tile_x: x,
            tile_y: y,
            tile_z: 0,
            size_x: size_x.max(1),
            size_y: size_y.max(1),
            size_z: 1,
            tile_size_x: 1,
            tile_size_y: 1,
            tile_size_z: 1,
        }
    }

    /// Creates a 3D volumetric CPU execution point for coordinate `(x, y, z)` within domain size `(size_x, size_y, size_z)`.
    ///
    /// Can be chained with [`.tile()`](Self::tile) to configure local tile coordinates.
    #[inline(always)]
    pub fn cpu_xyz(
        x: usize,
        y: usize,
        z: usize,
        size_x: usize,
        size_y: usize,
        size_z: usize,
    ) -> Self {
        Self {
            x,
            y,
            z,
            cell_x: 0,
            cell_y: 0,
            cell_z: 0,
            tile_x: x,
            tile_y: y,
            tile_z: z,
            size_x: size_x.max(1),
            size_y: size_y.max(1),
            size_z: size_z.max(1),
            tile_size_x: 1,
            tile_size_y: 1,
            tile_size_z: 1,
        }
    }
}
