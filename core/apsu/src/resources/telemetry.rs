#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuMemoryStats {
    pub total_vram_bytes: u64,
    pub used_vram_bytes: u64,
    pub available_vram_bytes: u64,
}

impl GpuMemoryStats {
    #[inline(always)]
    pub fn total_mb(&self) -> f64 {
        self.total_vram_bytes as f64 / (1024.0 * 1024.0)
    }

    #[inline(always)]
    pub fn total_gb(&self) -> f64 {
        self.total_vram_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }

    #[inline(always)]
    pub fn used_mb(&self) -> f64 {
        self.used_vram_bytes as f64 / (1024.0 * 1024.0)
    }

    #[inline(always)]
    pub fn used_gb(&self) -> f64 {
        self.used_vram_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }

    #[inline(always)]
    pub fn available_mb(&self) -> f64 {
        self.available_vram_bytes as f64 / (1024.0 * 1024.0)
    }

    #[inline(always)]
    pub fn available_gb(&self) -> f64 {
        self.available_vram_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }

    #[inline(always)]
    pub fn can_allocate(&self, required_bytes: u64) -> bool {
        required_bytes <= self.available_vram_bytes
    }
}
