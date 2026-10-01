use super::ro::Slice;
use super::rw::SliceMut;
use crate::enki_api::type_safety::GpuTypeMatch;
use anu::nam_args_api::{
    AccessIntent, ArgDescriptor, ArgValue, GpuBufferTarget, GpuType, IngressContext,
    InputResourceRecord, ResourceAccessKind,
};

impl<T: Copy + Send + Sync + 'static> GpuType for &Slice<'_, T> {
    fn describe() -> ArgDescriptor {
        ArgDescriptor::global_slice_read::<T>(false)
    }

    fn collect<'a>(&self, ctx: &mut IngressContext<'a>) {
        let desc = <Self as GpuType>::describe();
        ctx.push_arg(
            ArgValue::SliceBDA {
                bda: self.device_address,
                count: self.len as u64,
            },
            desc,
        );
        ctx.push_resource_binding(
            format!("Slice_Slot_{}", self.slot_index),
            self.slot_index,
            self.state.clone(),
            AccessIntent::Read,
        );
    }
}

impl<T: Copy + Send + Sync + 'static> GpuType for &mut SliceMut<'_, T> {
    fn describe() -> ArgDescriptor {
        ArgDescriptor::global_slice_read_write::<T>(false)
    }

    fn collect<'a>(&self, ctx: &mut IngressContext<'a>) {
        let desc = <Self as GpuType>::describe();
        ctx.push_arg(
            ArgValue::SliceBDA {
                bda: self.device_address,
                count: self.len as u64,
            },
            desc,
        );
        ctx.push_resource_binding(
            format!("SliceMut_Slot_{}", self.slot_index),
            self.slot_index,
            self.state.clone(),
            AccessIntent::ReadWrite,
        );
    }
}

impl<T: Copy + Send + Sync + 'static> GpuBufferTarget<T> for Slice<'_, T> {}
impl<T: Copy + Send + Sync + 'static> GpuBufferTarget<T> for SliceMut<'_, T> {}

impl<'target, T: Copy + Send + Sync + 'static> GpuTypeMatch<'target> for &'target Slice<'_, T> {
    type Target = &'target [T];

    fn describe() -> ArgDescriptor {
        ArgDescriptor::global_slice_read::<T>(false)
    }

    fn collect<'a>(&self, ctx: &mut IngressContext<'a>) {
        let desc = <Self as GpuTypeMatch>::describe();
        ctx.push_arg(
            ArgValue::SliceBDA {
                bda: self.device_address(),
                count: self.len as u64,
            },
            desc,
        );
        ctx.push_resource_binding(
            format!("Slice_Slot_{}", self.slot_index),
            self.slot_index,
            self.state.clone(),
            AccessIntent::Read,
        );
    }

    fn as_input_record(&self, arg_index: usize) -> InputResourceRecord {
        InputResourceRecord {
            arg_index,
            root_id: Some(self.root_id()),
            access_kind: ResourceAccessKind::Slice {
                element_range: self.element_range(),
                total_container_len: self.len,
            },
            is_mutable: false,
            element_type_name: std::any::type_name::<T>(),
        }
    }
}

impl<'target, T: Copy + Send + Sync + 'static> GpuTypeMatch<'target>
    for &'target mut SliceMut<'_, T>
{
    type Target = &'target mut [T];

    fn describe() -> ArgDescriptor {
        ArgDescriptor::global_slice_read_write::<T>(false)
    }

    fn collect<'a>(&self, ctx: &mut IngressContext<'a>) {
        let desc = <Self as GpuTypeMatch>::describe();
        ctx.push_arg(
            ArgValue::SliceBDA {
                bda: self.device_address(),
                count: self.len as u64,
            },
            desc,
        );
        ctx.push_resource_binding(
            format!("SliceMut_Slot_{}", self.slot_index),
            self.slot_index,
            self.state.clone(),
            AccessIntent::ReadWrite,
        );
    }

    fn as_input_record(&self, arg_index: usize) -> InputResourceRecord {
        InputResourceRecord {
            arg_index,
            root_id: Some(self.root_id()),
            access_kind: ResourceAccessKind::Slice {
                element_range: self.element_range(),
                total_container_len: self.len,
            },
            is_mutable: true,
            element_type_name: std::any::type_name::<T>(),
        }
    }
}

impl<'target, T: Copy + Send + Sync + 'static> GpuTypeMatch<'target> for &'target SliceMut<'_, T> {
    type Target = &'target [T];

    fn describe() -> ArgDescriptor {
        ArgDescriptor::global_slice_read::<T>(false)
    }

    fn collect<'a>(&self, ctx: &mut IngressContext<'a>) {
        let desc = <Self as GpuTypeMatch>::describe();
        ctx.push_arg(
            ArgValue::SliceBDA {
                bda: self.device_address(),
                count: self.len as u64,
            },
            desc,
        );
        ctx.push_resource_binding(
            format!("SliceMut_Slot_{}", self.slot_index),
            self.slot_index,
            self.state.clone(),
            AccessIntent::Read,
        );
    }

    fn as_input_record(&self, arg_index: usize) -> InputResourceRecord {
        InputResourceRecord {
            arg_index,
            root_id: Some(self.root_id()),
            access_kind: ResourceAccessKind::Slice {
                element_range: self.element_range(),
                total_container_len: self.len,
            },
            is_mutable: false,
            element_type_name: std::any::type_name::<T>(),
        }
    }
}
