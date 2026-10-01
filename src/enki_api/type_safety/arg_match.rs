use std::mem::size_of;

use crate::enki_api::resources::atomic::GpuAtomicTarget;
use crate::enki_api::resources::{GpuAtomic, GpuAtomicVec, GpuParam, GpuTileMem};
use anu::nam_args_api::{
    AccessIntent, ArgDescriptor, ArgValue, IngressContext, InputResourceRecord, ResourceAccessKind,
};

pub trait GpuTypeMatch<'target> {
    type Target;
    fn describe() -> ArgDescriptor;
    fn collect<'a>(&self, ctx: &mut IngressContext<'a>);
    fn as_input_record(&self, arg_index: usize) -> InputResourceRecord;
}

// 1. GpuParam
impl<'target, T: Copy + Send + Sync + 'static> GpuTypeMatch<'target> for GpuParam<T> {
    type Target = T;
    fn describe() -> ArgDescriptor {
        ArgDescriptor::by_value::<T>()
    }

    fn collect<'a>(&self, ctx: &mut IngressContext<'a>) {
        let size = size_of::<T>();
        let bytes =
            unsafe { std::slice::from_raw_parts(&self.value as *const T as *const u8, size) }
                .to_vec();
        ctx.push_arg(ArgValue::Payload(bytes), Self::describe());
    }

    fn as_input_record(&self, arg_index: usize) -> InputResourceRecord {
        InputResourceRecord {
            arg_index,
            root_id: None,
            access_kind: ResourceAccessKind::ValueUniform {
                byte_size: size_of::<T>(),
            },
            is_mutable: false,
            element_type_name: std::any::type_name::<T>(),
        }
    }
}

impl<'target, T: Copy + Send + Sync + 'static> GpuTypeMatch<'target> for &GpuParam<T> {
    type Target = T;
    fn describe() -> ArgDescriptor {
        ArgDescriptor::by_value::<T>()
    }

    fn collect<'a>(&self, ctx: &mut IngressContext<'a>) {
        GpuTypeMatch::collect(*self, ctx);
    }

    fn as_input_record(&self, arg_index: usize) -> InputResourceRecord {
        GpuTypeMatch::as_input_record(*self, arg_index)
    }
}

// 2. GpuTileMem
impl<'target, T: Copy + Send + Sync + 'static, const N: usize> GpuTypeMatch<'target>
    for GpuTileMem<T, N>
{
    type Target = &'target mut [T; N];
    fn describe() -> ArgDescriptor {
        ArgDescriptor::workgroup_scratchpad::<T, N>()
    }

    fn collect<'a>(&self, ctx: &mut IngressContext<'a>) {
        ctx.push_arg(ArgValue::ZeroFootprint, Self::describe());
    }

    fn as_input_record(&self, arg_index: usize) -> InputResourceRecord {
        InputResourceRecord {
            arg_index,
            root_id: None,
            access_kind: ResourceAccessKind::TileScratchpad { element_count: N },
            is_mutable: true,
            element_type_name: std::any::type_name::<T>(),
        }
    }
}

impl<'target, T: Copy + Send + Sync + 'static, const N: usize> GpuTypeMatch<'target>
    for &'target GpuTileMem<T, N>
{
    type Target = &'target mut [T; N];
    fn describe() -> ArgDescriptor {
        ArgDescriptor::workgroup_scratchpad::<T, N>()
    }

    fn collect<'a>(&self, ctx: &mut IngressContext<'a>) {
        GpuTypeMatch::collect(*self, ctx);
    }

    fn as_input_record(&self, arg_index: usize) -> InputResourceRecord {
        GpuTypeMatch::as_input_record(*self, arg_index)
    }
}

// 3. Atomics
impl<'target, T: GpuAtomicTarget> GpuTypeMatch<'target> for &'target GpuAtomic<T> {
    type Target = &'target T::NamTarget;
    fn describe() -> ArgDescriptor {
        ArgDescriptor::atomic_cell::<T>()
    }

    fn collect<'a>(&self, ctx: &mut IngressContext<'a>) {
        ctx.push_arg(ArgValue::BufferBDA(self.device_address), Self::describe());
        ctx.push_resource_binding(
            format!("GpuAtomic_Slot_{}", self.slot_index),
            self.slot_index,
            self.state.clone(),
            AccessIntent::ReadWrite,
        );
    }

    fn as_input_record(&self, arg_index: usize) -> InputResourceRecord {
        InputResourceRecord {
            arg_index,
            root_id: Some(self.slot_index as usize),
            access_kind: ResourceAccessKind::Atomic { element_count: 1 },
            is_mutable: true,
            element_type_name: std::any::type_name::<T>(),
        }
    }
}

impl<'target, T: GpuAtomicTarget> GpuTypeMatch<'target> for &'target mut GpuAtomic<T> {
    type Target = &'target T::NamTarget;

    fn describe() -> ArgDescriptor {
        ArgDescriptor::atomic_cell::<T>()
    }

    fn collect<'a>(&self, ctx: &mut IngressContext<'a>) {
        ctx.push_arg(ArgValue::BufferBDA(self.device_address), Self::describe());
        ctx.push_resource_binding(
            format!("GpuAtomic_Slot_{}", self.slot_index),
            self.slot_index,
            self.state.clone(),
            AccessIntent::ReadWrite,
        );
    }

    fn as_input_record(&self, arg_index: usize) -> InputResourceRecord {
        InputResourceRecord {
            arg_index,
            root_id: Some(self.slot_index as usize),
            access_kind: ResourceAccessKind::Atomic { element_count: 1 },
            is_mutable: true,
            element_type_name: std::any::type_name::<T>(),
        }
    }
}

impl<'target, T: GpuAtomicTarget> GpuTypeMatch<'target> for &'target GpuAtomicVec<T> {
    type Target = &'target [T::NamTarget];
    fn describe() -> ArgDescriptor {
        ArgDescriptor::atomic_slice::<T>()
    }

    fn collect<'a>(&self, ctx: &mut IngressContext<'a>) {
        ctx.push_arg(
            ArgValue::SliceBDA {
                bda: self.device_address,
                count: self.element_count as u64,
            },
            Self::describe(),
        );
        ctx.push_resource_binding(
            format!("GpuAtomicVec_Slot_{}", self.slot_index),
            self.slot_index,
            self.state.clone(),
            AccessIntent::ReadWrite,
        );
    }

    fn as_input_record(&self, arg_index: usize) -> InputResourceRecord {
        InputResourceRecord {
            arg_index,
            root_id: Some(self.slot_index as usize),
            access_kind: ResourceAccessKind::Atomic {
                element_count: self.element_count,
            },
            is_mutable: true,
            element_type_name: std::any::type_name::<T>(),
        }
    }
}

impl<'target, T: GpuAtomicTarget> GpuTypeMatch<'target> for &'target mut GpuAtomicVec<T> {
    type Target = &'target [T::NamTarget];

    fn describe() -> ArgDescriptor {
        ArgDescriptor::atomic_slice::<T>()
    }

    fn collect<'a>(&self, ctx: &mut IngressContext<'a>) {
        ctx.push_arg(
            ArgValue::SliceBDA {
                bda: self.device_address,
                count: self.element_count as u64,
            },
            Self::describe(),
        );
        ctx.push_resource_binding(
            format!("GpuAtomicVec_Slot_{}", self.slot_index),
            self.slot_index,
            self.state.clone(),
            AccessIntent::ReadWrite,
        );
    }

    fn as_input_record(&self, arg_index: usize) -> InputResourceRecord {
        InputResourceRecord {
            arg_index,
            root_id: Some(self.slot_index as usize),
            access_kind: ResourceAccessKind::Atomic {
                element_count: self.element_count,
            },
            is_mutable: true,
            element_type_name: std::any::type_name::<T>(),
        }
    }
}
