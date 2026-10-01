use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::AtomicU8;

use super::descriptor::{AccessIntent, ArgDescriptor};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgValue {
    Payload(Vec<u8>),
    BufferBDA(u64),
    SliceBDA { bda: u64, count: u64 },
    ZeroFootprint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvidedArg {
    pub value: ArgValue,
    pub descriptor: ArgDescriptor,
}

impl ProvidedArg {
    pub fn to_bytes(&self) -> Vec<u8> {
        match &self.value {
            ArgValue::Payload(bytes) => {
                let mut data = bytes.clone();
                let target_size = self.descriptor.arena_size_bytes;
                if data.len() < target_size {
                    data.resize(target_size, 0);
                }
                data
            }
            ArgValue::BufferBDA(bda) => bda.to_ne_bytes().to_vec(),
            ArgValue::SliceBDA { bda, count } => {
                let mut bytes = Vec::with_capacity(16);
                bytes.extend_from_slice(&bda.to_ne_bytes());
                bytes.extend_from_slice(&count.to_ne_bytes());
                bytes
            }
            ArgValue::ZeroFootprint => Vec::new(),
        }
    }
}

#[derive(Clone)]
pub struct BoundResource<'a> {
    pub name: String,
    pub slot_index: u32,
    pub state_atomic: Arc<AtomicU8>,
    pub intent: AccessIntent,
    pub arg_index: usize,
    pub _marker: PhantomData<&'a ()>,
}

pub struct IngressContext<'a> {
    pub args: Vec<ProvidedArg>,
    pub descriptors: Vec<ArgDescriptor>,
    pub bound_resources: Vec<BoundResource<'a>>,
    pub current_arg_index: usize,
}

impl<'a> IngressContext<'a> {
    pub fn new() -> Self {
        Self {
            args: Vec::with_capacity(8),
            descriptors: Vec::with_capacity(8),
            bound_resources: Vec::with_capacity(8),
            current_arg_index: 0,
        }
    }

    pub fn push_arg(&mut self, value: ArgValue, descriptor: ArgDescriptor) {
        self.descriptors.push(descriptor.clone());
        self.args.push(ProvidedArg { value, descriptor });
    }

    pub fn push_resource_binding(
        &mut self,
        name: String,
        slot_index: u32,
        state_atomic: Arc<AtomicU8>,
        intent: AccessIntent,
    ) {
        self.bound_resources.push(BoundResource {
            name,
            slot_index,
            state_atomic,
            intent,
            arg_index: self.current_arg_index,
            _marker: PhantomData,
        });
    }
    pub fn pack(&self) -> Vec<u8> {
        let total_size: usize = self.descriptors.iter().map(|d| d.arena_size_bytes).sum();
        let mut packed = Vec::with_capacity(total_size);

        for arg in &self.args {
            let bytes = arg.to_bytes();
            packed.extend_from_slice(&bytes);
        }

        packed
    }
}

impl<'a> Default for IngressContext<'a> {
    fn default() -> Self {
        Self::new()
    }
}
