use super::context::IngressContext;
use super::descriptor::ArgDescriptor;

pub trait GpuType {
    fn describe() -> ArgDescriptor;

    fn collect<'a>(&self, ctx: &mut IngressContext<'a>);
}

pub trait NamArgs {
    fn collect_all<'a>(self) -> IngressContext<'a>;

    fn describe_all() -> Vec<ArgDescriptor>;
}

pub trait GpuBufferTarget<T> {}
