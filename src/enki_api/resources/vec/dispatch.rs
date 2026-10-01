use super::core::GpuVec;
use crate::enki_api::type_safety::GpuTypeMatch;
use anu::nam_args_api::{
    AccessIntent, ArgDescriptor, ArgValue, GpuBufferTarget, GpuType, IngressContext,
    InputResourceRecord, ResourceAccessKind,
};

impl<T: Copy + Send + Sync + 'static> GpuType for &GpuVec<T> {
    fn describe() -> ArgDescriptor {
        ArgDescriptor::spmd_cell_const::<T>(false)
    }

    fn collect<'a>(&self, ctx: &mut IngressContext<'a>) {
        let desc = <Self as GpuType>::describe();
        ctx.push_arg(ArgValue::BufferBDA(self.device_address), desc);
        ctx.push_resource_binding(
            format!("GpuVec_Slot_{}", self.slot_index),
            self.slot_index,
            self.state.clone(),
            AccessIntent::Read,
        );
    }
}

impl<T: Copy + Send + Sync + 'static> GpuType for &mut GpuVec<T> {
    fn describe() -> ArgDescriptor {
        ArgDescriptor::spmd_cell_mut::<T>(false)
    }

    fn collect<'a>(&self, ctx: &mut IngressContext<'a>) {
        let desc = <Self as GpuType>::describe();
        ctx.push_arg(ArgValue::BufferBDA(self.device_address), desc);
        ctx.push_resource_binding(
            format!("GpuVec_Slot_{}", self.slot_index),
            self.slot_index,
            self.state.clone(),
            AccessIntent::ReadWrite,
        );
    }
}

impl<T: Copy + Send + Sync + 'static> GpuBufferTarget<T> for GpuVec<T> {}

impl<'target, T: Copy + Send + Sync + 'static> GpuTypeMatch<'target> for &'target GpuVec<T> {
    type Target = &'target T;

    fn describe() -> ArgDescriptor {
        ArgDescriptor::spmd_cell_const::<T>(false)
    }

    fn collect<'a>(&self, ctx: &mut IngressContext<'a>) {
        let desc = <Self as GpuTypeMatch>::describe();
        ctx.push_arg(ArgValue::BufferBDA(self.device_address()), desc);
        ctx.push_resource_binding(
            format!("GpuVec_Slot_{}", self.slot_index()),
            self.slot_index(),
            self.state.clone(),
            AccessIntent::Read,
        );
    }

    fn as_input_record(&self, arg_index: usize) -> InputResourceRecord {
        InputResourceRecord {
            arg_index,
            root_id: Some(self.slot_index() as usize),
            access_kind: ResourceAccessKind::PerCell {
                element_count: self.len,
            },
            is_mutable: false,
            element_type_name: std::any::type_name::<T>(),
        }
    }
}

impl<'target, T: Copy + Send + Sync + 'static> GpuTypeMatch<'target> for &'target mut GpuVec<T> {
    type Target = &'target mut T;

    fn describe() -> ArgDescriptor {
        ArgDescriptor::spmd_cell_mut::<T>(false)
    }

    fn collect<'a>(&self, ctx: &mut IngressContext<'a>) {
        let desc = <Self as GpuTypeMatch>::describe();
        ctx.push_arg(ArgValue::BufferBDA(self.device_address()), desc);
        ctx.push_resource_binding(
            format!("GpuVec_Slot_{}", self.slot_index()),
            self.slot_index(),
            self.state.clone(),
            AccessIntent::ReadWrite,
        );
    }

    fn as_input_record(&self, arg_index: usize) -> InputResourceRecord {
        InputResourceRecord {
            arg_index,
            root_id: Some(self.slot_index as usize),
            access_kind: ResourceAccessKind::PerCell {
                element_count: self.len,
            },
            is_mutable: true,
            element_type_name: std::any::type_name::<T>(),
        }
    }
}
