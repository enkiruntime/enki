use ash::vk;
use anyhow::{Result, Context};

pub struct VulkanDescriptorSetLayout {
    pub handle: vk::DescriptorSetLayout,
    device: ash::Device,
}

impl Drop for VulkanDescriptorSetLayout {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_descriptor_set_layout(self.handle, None);
        }
    }
}

pub struct DescriptorBinding {
    pub binding: u32,
    pub descriptor_type: vk::DescriptorType,
    pub count: u32,
    pub stage_flags: vk::ShaderStageFlags,
    pub binding_flags: vk::DescriptorBindingFlags,
}

pub struct DescriptorSetLayoutBuilder {
    bindings: Vec<DescriptorBinding>,
}

impl DescriptorSetLayoutBuilder {
    pub fn new() -> Self {
        Self { bindings: Vec::new() }
    }

    pub fn add_binding(
        mut self,
        binding: u32,
        descriptor_type: vk::DescriptorType,
        count: u32,
        stage_flags: vk::ShaderStageFlags,
        binding_flags: vk::DescriptorBindingFlags,
    ) -> Self {
        self.bindings.push(DescriptorBinding {
            binding,
            descriptor_type,
            count,
            stage_flags,
            binding_flags,
        });
        self
    }

    pub fn build(self, device: &ash::Device) -> Result<VulkanDescriptorSetLayout> {
        let vk_bindings: Vec<vk::DescriptorSetLayoutBinding> = self.bindings
            .iter()
            .map(|b| {
                vk::DescriptorSetLayoutBinding::default()
                    .binding(b.binding)
                    .descriptor_type(b.descriptor_type)
                    .descriptor_count(b.count)
                    .stage_flags(b.stage_flags)
            })
            .collect();

        let vk_flags: Vec<vk::DescriptorBindingFlags> = self.bindings
            .iter()
            .map(|b| b.binding_flags)
            .collect();

        let mut flags_info = vk::DescriptorSetLayoutBindingFlagsCreateInfo::default()
            .binding_flags(&vk_flags);

        let mut layout_flags = vk::DescriptorSetLayoutCreateFlags::empty();
        if vk_flags.iter().any(|f| f.contains(vk::DescriptorBindingFlags::UPDATE_AFTER_BIND)) {
            layout_flags |= vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL;
        }

        let layout_info = vk::DescriptorSetLayoutCreateInfo::default()
            .push_next(&mut flags_info)
            .flags(layout_flags)
            .bindings(&vk_bindings);

        let handle = unsafe {
            device.create_descriptor_set_layout(&layout_info, None)
                .context("[DescriptorSetLayoutBuilder] Failed to create raw DescriptorSetLayout")?
        };

        Ok(VulkanDescriptorSetLayout {
            handle,
            device: device.clone(),
        })
    }
}

pub struct VulkanDescriptorPool {
    pub handle: vk::DescriptorPool,
    device: ash::Device,
}

impl VulkanDescriptorPool {
    pub fn allocate_sets(
        &self,
        layout: vk::DescriptorSetLayout,
        count: u32,
    ) -> Result<Vec<vk::DescriptorSet>> {
        let layouts = vec![layout; count as usize];
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.handle)
            .set_layouts(&layouts);

        let sets = unsafe {
            self.device.allocate_descriptor_sets(&alloc_info)
                .context("[VulkanDescriptorPool] Failed to allocate Descriptor Sets")?
        };
        Ok(sets)
    }

    pub fn allocate_set(&self, layout: vk::DescriptorSetLayout) -> Result<vk::DescriptorSet> {
        let sets = self.allocate_sets(layout, 1)?;
        Ok(sets[0])
    }
}

impl Drop for VulkanDescriptorPool {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_descriptor_pool(self.handle, None);
        }
    }
}

pub struct DescriptorPoolBuilder {
    pool_sizes: Vec<vk::DescriptorPoolSize>,
    max_sets: u32,
    flags: vk::DescriptorPoolCreateFlags,
}

impl DescriptorPoolBuilder {
    pub fn new() -> Self {
        Self {
            pool_sizes: Vec::new(),
            max_sets: 1,
            flags: vk::DescriptorPoolCreateFlags::empty(),
        }
    }

    pub fn add_pool_size(mut self, ty: vk::DescriptorType, count: u32) -> Self {
        self.pool_sizes.push(vk::DescriptorPoolSize {
            ty,
            descriptor_count: count,
        });
        self
    }

    pub fn with_max_sets(mut self, max_sets: u32) -> Self {
        self.max_sets = max_sets;
        self
    }

    pub fn with_flags(mut self, flags: vk::DescriptorPoolCreateFlags) -> Self {
        self.flags = flags;
        self
    }

    pub fn build(self, device: &ash::Device) -> Result<VulkanDescriptorPool> {
        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .flags(self.flags)
            .max_sets(self.max_sets)
            .pool_sizes(&self.pool_sizes);

        let handle = unsafe {
            device.create_descriptor_pool(&pool_info, None)
                .context("[DescriptorPoolBuilder] Failed to create raw DescriptorPool")?
        };

        Ok(VulkanDescriptorPool {
            handle,
            device: device.clone(),
        })
    }
}