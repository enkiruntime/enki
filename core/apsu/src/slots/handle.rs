use std::sync::Arc;

use super::pool::SlotType;
use crate::resources::allocator::ApsuAllocator;

use std::sync::atomic::AtomicU8;

pub struct SlotHandle {
    pub slot_type: SlotType,
    pub index: u32,
    allocator: Arc<ApsuAllocator>,
    pub state: Arc<AtomicU8>,
}

impl SlotHandle {
    pub fn new(slot_type: SlotType, index: u32, allocator: Arc<ApsuAllocator>) -> Self {
        Self {
            slot_type,
            index,
            allocator,
            state: Arc::new(AtomicU8::new(0)),
        }
    }
}

impl Drop for SlotHandle {
    fn drop(&mut self) {
        let resource = crate::resources::allocator::DeletableResource::Slot {
            slot_type: self.slot_type,
            index: self.index,
        };
        self.allocator.defer_deletion(resource);
    }
}
