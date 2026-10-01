use ash::vk;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SlotType {
    SampledImage,
    StorageImage,
    Sampler,
}

#[derive(Debug, Clone)]
pub enum SlotEvent {
    BindImage {
        slot_type: SlotType,
        slot_index: u32,
        image_view: vk::ImageView,
        image_layout: vk::ImageLayout,
    },
    BindSampler {
        slot_index: u32,
        sampler: vk::Sampler,
    },
    Unbind {
        slot_type: SlotType,
        slot_index: u32,
    },
}

pub struct SlotPool {
    sampled_image_free_list: std::sync::Mutex<Vec<u32>>,
    storage_image_free_list: std::sync::Mutex<Vec<u32>>,
    sampler_free_list: std::sync::Mutex<Vec<u32>>,
}

impl SlotPool {
    pub fn new(max_sampled_image: u32, max_storage_image: u32, max_sampler: u32) -> Self {
        let mut sampled: Vec<u32> = (0..max_sampled_image).collect();
        sampled.reverse();

        let mut storage_img: Vec<u32> = (0..max_storage_image).collect();
        storage_img.reverse();

        let mut sampler: Vec<u32> = (0..max_sampler).collect();
        sampler.reverse();

        Self {
            sampled_image_free_list: std::sync::Mutex::new(sampled),
            storage_image_free_list: std::sync::Mutex::new(storage_img),
            sampler_free_list: std::sync::Mutex::new(sampler),
        }
    }

    pub fn allocate(&self, slot_type: SlotType) -> Result<u32, String> {
        let mut list = match slot_type {
            SlotType::SampledImage => self
                .sampled_image_free_list
                .lock()
                .map_err(|e| e.to_string())?,
            SlotType::StorageImage => self
                .storage_image_free_list
                .lock()
                .map_err(|e| e.to_string())?,
            SlotType::Sampler => self.sampler_free_list.lock().map_err(|e| e.to_string())?,
        };

        list.pop()
            .ok_or_else(|| format!("[SlotPool] No free slots left for {:?}", slot_type))
    }

    pub fn free(&self, slot_type: SlotType, index: u32) -> Result<(), String> {
        let mut list = match slot_type {
            SlotType::SampledImage => self
                .sampled_image_free_list
                .lock()
                .map_err(|e| e.to_string())?,
            SlotType::StorageImage => self
                .storage_image_free_list
                .lock()
                .map_err(|e| e.to_string())?,
            SlotType::Sampler => self.sampler_free_list.lock().map_err(|e| e.to_string())?,
        };

        list.push(index);
        Ok(())
    }
}
