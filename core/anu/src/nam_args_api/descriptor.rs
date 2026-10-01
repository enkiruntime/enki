use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AccessIntent {
    Read,
    Write,
    ReadWrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryDomain {
    ArenaPayload,
    PhysicalStorageBuffer,
    WorkgroupShared,
    ZeroSized,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AccessMode {
    ByValue,
    SpmdCellMut,
    SpmdCellConst,
    GlobalSliceRead,
    GlobalSliceReadWrite,
    AtomicCell,
    AtomicSlice,
    WorkgroupScratchpad,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ArgDescriptor {
    pub mode: AccessMode,
    pub domain: MemoryDomain,
    pub intent: AccessIntent,
    pub is_optional: bool,
    pub element_size: usize,
    pub stride: usize,
    pub alignment: usize,
    pub arena_size_bytes: usize,
}

impl ArgDescriptor {
    pub fn by_value<T: 'static>() -> Self {
        let size = std::mem::size_of::<T>();
        let align = std::mem::align_of::<T>();
        if size == 0 {
            return Self::zero_sized();
        }
        let arena_size = (size + 7) & !7;

        Self {
            mode: AccessMode::ByValue,
            domain: MemoryDomain::ArenaPayload,
            intent: AccessIntent::Read,
            is_optional: false,
            element_size: size,
            stride: size,
            alignment: align,
            arena_size_bytes: arena_size,
        }
    }

    pub fn spmd_cell_mut<T: 'static>(is_optional: bool) -> Self {
        let size = std::mem::size_of::<T>();
        let align = std::mem::align_of::<T>();

        Self {
            mode: AccessMode::SpmdCellMut,
            domain: MemoryDomain::PhysicalStorageBuffer,
            intent: AccessIntent::ReadWrite,
            is_optional,
            element_size: size,
            stride: size,
            alignment: align,
            arena_size_bytes: 8,
        }
    }

    pub fn spmd_cell_const<T: 'static>(is_optional: bool) -> Self {
        let size = std::mem::size_of::<T>();
        let align = std::mem::align_of::<T>();

        Self {
            mode: AccessMode::SpmdCellConst,
            domain: MemoryDomain::PhysicalStorageBuffer,
            intent: AccessIntent::Read,
            is_optional,
            element_size: size,
            stride: size,
            alignment: align,
            arena_size_bytes: 8,
        }
    }

    pub fn global_slice_read<T: 'static>(is_optional: bool) -> Self {
        let size = std::mem::size_of::<T>();
        let align = std::mem::align_of::<T>();

        Self {
            mode: AccessMode::GlobalSliceRead,
            domain: MemoryDomain::PhysicalStorageBuffer,
            intent: AccessIntent::Read,
            is_optional,
            element_size: size,
            stride: size,
            alignment: align,
            arena_size_bytes: 16,
        }
    }

    pub fn global_slice_read_write<T: 'static>(is_optional: bool) -> Self {
        let size = std::mem::size_of::<T>();
        let align = std::mem::align_of::<T>();

        Self {
            mode: AccessMode::GlobalSliceReadWrite,
            domain: MemoryDomain::PhysicalStorageBuffer,
            intent: AccessIntent::ReadWrite,
            is_optional,
            element_size: size,
            stride: size,
            alignment: align,
            arena_size_bytes: 16,
        }
    }

    pub fn atomic_cell<T: 'static>() -> Self {
        let size = std::mem::size_of::<T>();
        let align = std::mem::align_of::<T>();

        Self {
            mode: AccessMode::AtomicCell,
            domain: MemoryDomain::PhysicalStorageBuffer,
            intent: AccessIntent::ReadWrite,
            is_optional: false,
            element_size: size,
            stride: size,
            alignment: align,
            arena_size_bytes: 8,
        }
    }

    pub fn atomic_slice<T: 'static>() -> Self {
        let size = std::mem::size_of::<T>();
        let align = std::mem::align_of::<T>();

        Self {
            mode: AccessMode::AtomicSlice,
            domain: MemoryDomain::PhysicalStorageBuffer,
            intent: AccessIntent::ReadWrite,
            is_optional: false,
            element_size: size,
            stride: size,
            alignment: align,
            arena_size_bytes: 16,
        }
    }

    pub fn workgroup_scratchpad<T: 'static, const N: usize>() -> Self {
        let size = std::mem::size_of::<T>();
        let align = std::mem::align_of::<T>();

        Self {
            mode: AccessMode::WorkgroupScratchpad,
            domain: MemoryDomain::WorkgroupShared,
            intent: AccessIntent::ReadWrite,
            is_optional: false,
            element_size: size * N,
            stride: size,
            alignment: align,
            arena_size_bytes: 0,
        }
    }

    pub fn zero_sized() -> Self {
        Self {
            mode: AccessMode::ByValue,
            domain: MemoryDomain::ZeroSized,
            intent: AccessIntent::Read,
            is_optional: false,
            element_size: 0,
            stride: 0,
            alignment: 1,
            arena_size_bytes: 0,
        }
    }
}
