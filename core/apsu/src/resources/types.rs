#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryUsage {
    DeviceOnly,
    Upload,
    Download,
    ZeroCopy,
}

impl MemoryUsage {
    pub fn to_vma(&self, is_unified_memory: bool) -> (vk_mem::MemoryUsage, vk_mem::AllocationCreateFlags) {
        match self {
            MemoryUsage::DeviceOnly => (
                vk_mem::MemoryUsage::AutoPreferDevice,
                vk_mem::AllocationCreateFlags::empty(),
            ),
            MemoryUsage::Upload => (
                vk_mem::MemoryUsage::AutoPreferHost,
                vk_mem::AllocationCreateFlags::MAPPED | vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
            ),
            MemoryUsage::Download => (
                vk_mem::MemoryUsage::AutoPreferHost,
                vk_mem::AllocationCreateFlags::MAPPED | vk_mem::AllocationCreateFlags::HOST_ACCESS_RANDOM,
            ),
            MemoryUsage::ZeroCopy => {
                if is_unified_memory {
                    (
                        vk_mem::MemoryUsage::AutoPreferDevice,
                        vk_mem::AllocationCreateFlags::MAPPED | vk_mem::AllocationCreateFlags::HOST_ACCESS_RANDOM,
                    )
                } else {
                    (
                        vk_mem::MemoryUsage::AutoPreferHost,
                        vk_mem::AllocationCreateFlags::MAPPED | vk_mem::AllocationCreateFlags::HOST_ACCESS_RANDOM,
                    )
                }
            }
        }
    }
}

pub type BufferUsage = ash::vk::BufferUsageFlags;