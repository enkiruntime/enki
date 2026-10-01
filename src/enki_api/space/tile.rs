/// Configuration strategy for partitioning execution domains into spatial tiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TileConfig {
    /// Automatic tile sizing derived dynamically from device hardware specifications.
    #[default]
    Auto,
    /// Explicit tile dimensions along the X, Y, and Z axes: `(tile_x, tile_y, tile_z)`.
    Custom(u32, u32, u32),
}

impl TileConfig {
    /// Creates an explicit 1D tile configuration along the X axis.
    #[inline(always)]
    pub const fn custom_x(x: u32) -> Self {
        Self::Custom(x, 1, 1)
    }

    /// Creates an explicit 2D tile configuration along the X and Y axes.
    #[inline(always)]
    pub const fn custom_xy(x: u32, y: u32) -> Self {
        Self::Custom(x, y, 1)
    }

    /// Creates an explicit 3D volumetric tile configuration along the X, Y, and Z axes.
    #[inline(always)]
    pub const fn custom_xyz(x: u32, y: u32, z: u32) -> Self {
        Self::Custom(x, y, z)
    }

    /// Alias for [`Self::custom_x`].
    #[inline(always)]
    pub const fn custom_1d(x: u32) -> Self {
        Self::custom_x(x)
    }

    /// Alias for [`Self::custom_xy`].
    #[inline(always)]
    pub const fn custom_2d(x: u32, y: u32) -> Self {
        Self::custom_xy(x, y)
    }

    /// Alias for [`Self::custom_xyz`].
    #[inline(always)]
    pub const fn custom_3d(x: u32, y: u32, z: u32) -> Self {
        Self::custom_xyz(x, y, z)
    }

    /// Returns `true` if this configuration is set to automatic hardware tuning.
    #[inline(always)]
    pub const fn is_auto(&self) -> bool {
        matches!(self, Self::Auto)
    }
}

/// Conversion trait enabling ergonomic construction of [`TileConfig`] from numbers, tuples, and arrays.
pub trait IntoTile {
    /// Converts this value into a concrete [`TileConfig`].
    fn into_tile(self) -> TileConfig;
}

impl IntoTile for TileConfig {
    #[inline(always)]
    fn into_tile(self) -> TileConfig {
        self
    }
}

impl IntoTile for u32 {
    #[inline(always)]
    fn into_tile(self) -> TileConfig {
        TileConfig::Custom(self.max(1), 1, 1)
    }
}

impl IntoTile for i32 {
    #[inline(always)]
    fn into_tile(self) -> TileConfig {
        TileConfig::Custom(self.max(1) as u32, 1, 1)
    }
}

impl IntoTile for usize {
    #[inline(always)]
    fn into_tile(self) -> TileConfig {
        TileConfig::Custom((self as u32).max(1), 1, 1)
    }
}

impl IntoTile for [u32; 2] {
    #[inline(always)]
    fn into_tile(self) -> TileConfig {
        TileConfig::Custom(self[0].max(1), self[1].max(1), 1)
    }
}

impl IntoTile for (u32, u32) {
    #[inline(always)]
    fn into_tile(self) -> TileConfig {
        TileConfig::Custom(self.0.max(1), self.1.max(1), 1)
    }
}

impl IntoTile for [i32; 2] {
    #[inline(always)]
    fn into_tile(self) -> TileConfig {
        TileConfig::Custom(self[0].max(1) as u32, self[1].max(1) as u32, 1)
    }
}

impl IntoTile for (i32, i32) {
    #[inline(always)]
    fn into_tile(self) -> TileConfig {
        TileConfig::Custom(self.0.max(1) as u32, self.1.max(1) as u32, 1)
    }
}

impl IntoTile for [usize; 2] {
    #[inline(always)]
    fn into_tile(self) -> TileConfig {
        TileConfig::Custom((self[0] as u32).max(1), (self[1] as u32).max(1), 1)
    }
}

impl IntoTile for (usize, usize) {
    #[inline(always)]
    fn into_tile(self) -> TileConfig {
        TileConfig::Custom((self.0 as u32).max(1), (self.1 as u32).max(1), 1)
    }
}

impl IntoTile for [u32; 3] {
    #[inline(always)]
    fn into_tile(self) -> TileConfig {
        TileConfig::Custom(self[0].max(1), self[1].max(1), self[2].max(1))
    }
}

impl IntoTile for (u32, u32, u32) {
    #[inline(always)]
    fn into_tile(self) -> TileConfig {
        TileConfig::Custom(self.0.max(1), self.1.max(1), self.2.max(1))
    }
}

impl IntoTile for [i32; 3] {
    #[inline(always)]
    fn into_tile(self) -> TileConfig {
        TileConfig::Custom(
            self[0].max(1) as u32,
            self[1].max(1) as u32,
            self[2].max(1) as u32,
        )
    }
}

impl IntoTile for (i32, i32, i32) {
    #[inline(always)]
    fn into_tile(self) -> TileConfig {
        TileConfig::Custom(
            self.0.max(1) as u32,
            self.1.max(1) as u32,
            self.2.max(1) as u32,
        )
    }
}

impl IntoTile for [usize; 3] {
    #[inline(always)]
    fn into_tile(self) -> TileConfig {
        TileConfig::Custom(
            (self[0] as u32).max(1),
            (self[1] as u32).max(1),
            (self[2] as u32).max(1),
        )
    }
}

impl IntoTile for (usize, usize, usize) {
    #[inline(always)]
    fn into_tile(self) -> TileConfig {
        TileConfig::Custom(
            (self.0 as u32).max(1),
            (self.1 as u32).max(1),
            (self.2 as u32).max(1),
        )
    }
}
