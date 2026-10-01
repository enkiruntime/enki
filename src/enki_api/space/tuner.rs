//! Module: enki_api/space/tuner.rs
//!
//! Hardware-agnostic mathematical engine for deriving optimal spatial tile dimensions.

use super::tile::TileConfig;
use parsu::profile::HardwareProfile;

pub struct SpatialAutoTuner;

impl SpatialAutoTuner {
    /// Resolves the final (tile_x, tile_y, tile_z) based on the requested TileConfig,
    /// problem dimensions, and the queried hardware profile.
    pub fn resolve_tile(
        config: TileConfig,
        global_size: (u32, u32, u32),
        profile: &HardwareProfile,
    ) -> (u32, u32, u32) {
        let (gx, gy, gz) = (
            global_size.0.max(1),
            global_size.1.max(1),
            global_size.2.max(1),
        );

        match config {
            TileConfig::Custom(cx, cy, cz) => Self::validate_and_clamp_custom(cx, cy, cz, profile),
            TileConfig::Auto => {
                let s = if profile.subgroup_size == 0 {
                    32
                } else {
                    profile.subgroup_size
                };
                let m = if profile.max_compute_workgroup_invocations == 0 {
                    1024
                } else {
                    profile.max_compute_workgroup_invocations
                };

                if gz > 1 {
                    Self::tune_3d(gx, gy, gz, s, m, profile.is_integrated)
                } else if gy > 1 {
                    Self::tune_2d(gx, gy, s, m, profile.is_integrated)
                } else {
                    Self::tune_1d(gx, s, m, profile.is_integrated)
                }
            }
        }
    }

    /// Computes the target invocation count per workgroup (T) based on subgroup size.
    /// Latency hiding sweet spot is between 4 and 8 subgroups, clamped by hardware limit (M).
    #[inline(always)]
    fn target_invocations(s: u32, m: u32, is_integrated: bool) -> u32 {
        if is_integrated {
            (2 * s).min(m).min(64)
        } else {
            let ideal = (8 * s).min(m);
            if s <= 32 {
                ideal.min(256)
            } else {
                ideal.min(512)
            }
        }
    }

    /// 1D Auto-tuning: Aligns to subgroup size (S) and avoids trailing inactive threads for small N.
    fn tune_1d(n: u32, s: u32, m: u32, is_integrated: bool) -> (u32, u32, u32) {
        let target = Self::target_invocations(s, m, is_integrated);

        // If the problem size N is smaller than the target, allocate the smallest multiple of S
        let needed_subgroups = (n + s - 1) / s;
        let needed_threads = (needed_subgroups * s).max(s);

        let final_x = target.min(needed_threads).min(m);
        (final_x, 1, 1)
    }

    /// 2D Auto-tuning: Geometric 2D factorization (Lx * Ly = T) optimizing L1/L2 Spatial Locality.
    fn tune_2d(_w: u32, _h: u32, s: u32, m: u32, is_integrated: bool) -> (u32, u32, u32) {
        let target = Self::target_invocations(s, m, is_integrated);

        // Decompose target T into (Lx, Ly) power-of-two rectangles where Lx >= Ly
        let ly = 1u32 << (target.ilog2() / 2);
        let lx = target / ly;

        (lx, ly, 1)
    }

    /// 3D Auto-tuning: Volumetric factorization (Lx * Ly * Lz = T).
    fn tune_3d(_x: u32, _y: u32, _z: u32, s: u32, m: u32, is_integrated: bool) -> (u32, u32, u32) {
        let target = Self::target_invocations(s, m, is_integrated);

        let lz = 1u32 << (target.ilog2() / 3);
        let remainder = target / lz;
        let ly = 1u32 << (remainder.ilog2() / 2);
        let lx = remainder / ly;

        (lx, ly, lz)
    }

    /// Validates user custom tiles against physical device limits.
    fn validate_and_clamp_custom(
        cx: u32,
        cy: u32,
        cz: u32,
        profile: &HardwareProfile,
    ) -> (u32, u32, u32) {
        let x = cx.max(1);
        let y = cy.max(1);
        let z = cz.max(1);

        let total = (x as u64) * (y as u64) * (z as u64);
        let m = if profile.max_compute_workgroup_invocations == 0 {
            1024
        } else {
            profile.max_compute_workgroup_invocations as u64
        };

        if total > m {
            let diag = anu::diagnostics::hw::tile_invocations_exceeded(None);
            crate::enki_api::context::errors::emit_warning(&diag);

            let clamped_x = ((m / ((y * z) as u64)) as u32).max(1);
            (clamped_x, y, z)
        } else {
            (x, y, z)
        }
    }
}
