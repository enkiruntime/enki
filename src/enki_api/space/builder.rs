use super::core::Space;
use super::tile::{IntoTile, TileConfig};
use super::tuner::SpatialAutoTuner;
use parsu::profile::HardwareProfile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ResolvedSpatialDispatch {
    pub global_size: (u32, u32, u32),
    pub local_size: (u32, u32, u32),
    pub dispatch_groups: (u32, u32, u32),
}

impl Space {
    /// Creates a 1D GPU execution domain with the specified global problem size.
    ///
    /// By default, tile dimensions are automatically factored into optimal workgroups
    #[inline(always)]
    pub fn gpu_x(size_x: usize) -> Self {
        Self {
            x: 0,
            y: 0,
            z: 0,
            cell_x: 0,
            cell_y: 0,
            cell_z: 0,
            tile_x: 0,
            tile_y: 0,
            tile_z: 0,
            size_x: size_x.max(1),
            size_y: 1,
            size_z: 1,
            tile_size_x: 0,
            tile_size_y: 0,
            tile_size_z: 0,
        }
    }

    /// Creates a 2D GPU execution domain with the specified width and height.
    ///
    /// By default, tile dimensions are automatically factored into optimal workgroups.
    #[inline(always)]
    pub fn gpu_xy(size_x: usize, size_y: usize) -> Self {
        Self {
            x: 0,
            y: 0,
            z: 0,
            cell_x: 0,
            cell_y: 0,
            cell_z: 0,
            tile_x: 0,
            tile_y: 0,
            tile_z: 0,
            size_x: size_x.max(1),
            size_y: size_y.max(1),
            size_z: 1,
            tile_size_x: 0,
            tile_size_y: 0,
            tile_size_z: 0,
        }
    }

    /// Creates a 3D volumetric GPU execution domain with the specified width, height, and depth.
    ///
    /// By default, tile dimensions are automatically factored into optimal workgroups.
    #[inline(always)]
    pub fn gpu_xyz(size_x: usize, size_y: usize, size_z: usize) -> Self {
        Self {
            x: 0,
            y: 0,
            z: 0,
            cell_x: 0,
            cell_y: 0,
            cell_z: 0,
            tile_x: 0,
            tile_y: 0,
            tile_z: 0,
            size_x: size_x.max(1),
            size_y: size_y.max(1),
            size_z: size_z.max(1),
            tile_size_x: 0,
            tile_size_y: 0,
            tile_size_z: 0,
        }
    }

    /// Overrides automatic tile tuning with an explicit, fixed tile configuration.
    ///
    /// Accepts scalar values for 1D, tuples/arrays for 2D, and triplets for 3D.
    ///
    /// # Examples
    /// ```rust
    /// use enki::Space;
    ///
    /// let space_1d = Space::gpu_x(10_000_000).tile(256);
    /// let space_2d = Space::gpu_xy(1920, 1080).tile([16, 16]);
    /// let space_3d = Space::gpu_xyz(256, 256, 128).tile([8, 8, 4]);
    /// ```
    #[inline(always)]
    pub fn tile<T: IntoTile>(mut self, tile: T) -> Self {
        match tile.into_tile() {
            TileConfig::Auto => {
                self.tile_size_x = 0;
                self.tile_size_y = 0;
                self.tile_size_z = 0;
            }
            TileConfig::Custom(tx, ty, tz) => {
                self.tile_size_x = tx as usize;
                self.tile_size_y = ty as usize;
                self.tile_size_z = tz as usize;

                if self.tile_size_x > 0 {
                    self.cell_x = self.x % self.tile_size_x;
                    self.tile_x = self.x / self.tile_size_x;
                }
                if self.tile_size_y > 0 {
                    self.cell_y = self.y % self.tile_size_y;
                    self.tile_y = self.y / self.tile_size_y;
                }
                if self.tile_size_z > 0 {
                    self.cell_z = self.z % self.tile_size_z;
                    self.tile_z = self.z / self.tile_size_z;
                }
            }
        }
        self
    }

    pub(crate) fn resolve_dispatch(&self, profile: &HardwareProfile) -> ResolvedSpatialDispatch {
        let global_size = (self.size_x as u32, self.size_y as u32, self.size_z as u32);

        let config = if self.tile_size_x == 0 {
            TileConfig::Auto
        } else {
            TileConfig::Custom(
                self.tile_size_x as u32,
                self.tile_size_y.max(1) as u32,
                self.tile_size_z.max(1) as u32,
            )
        };

        let local_size = SpatialAutoTuner::resolve_tile(config, global_size, profile);

        let group_x = global_size.0.div_ceil(local_size.0);
        let group_y = global_size.1.div_ceil(local_size.1);
        let group_z = global_size.2.div_ceil(local_size.2);

        ResolvedSpatialDispatch {
            global_size,
            local_size,
            dispatch_groups: (group_x, group_y, group_z),
        }
    }
}
