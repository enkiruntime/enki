use crate::context::EnkiEngine;
use anyhow::{Context, Result};
use ash::vk;
use std::ffi::CString;

pub struct ComputePipelineFactory;

impl ComputePipelineFactory {
    pub fn compile(
        engine: &EnkiEngine,
        spirv_bytes: &[u8],
        pipeline_cache: vk::PipelineCache,
    ) -> Result<(vk::Pipeline, vk::PipelineLayout)> {
        let device = engine.raw_device();

        let specialized_bytes = engine
            .driver_specializer
            .specialize_and_optimize(spirv_bytes, &engine.hardware_profile)
            .context(
                "[ComputePipelineFactory] Driver specialization/optimization of SPIR-V failed",
            )?;

        if specialized_bytes.len() % 4 != 0 {
            return Err(anyhow::anyhow!(
                "[ComputePipelineFactory] Specialized SPIR-V bytecode length must be a multiple of 4"
            ));
        }

        let words = bytemuck::cast_slice::<u8, u32>(&specialized_bytes);

        let shader_module_info = vk::ShaderModuleCreateInfo::default().code(words);

        let shader_module = unsafe {
            device
                .create_shader_module(&shader_module_info, None)
                .context("[ComputePipelineFactory] Failed to create Vulkan Shader Module")?
        };

        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::COMPUTE)
            .offset(0)
            .size(32);

        let set_layouts = [engine.descriptor_layout.handle];

        let layout_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&set_layouts)
            .push_constant_ranges(std::slice::from_ref(&push_constant_range));

        let pipeline_layout = unsafe {
            device
                .create_pipeline_layout(&layout_info, None)
                .context("[ComputePipelineFactory] Failed to create Vulkan Pipeline Layout")?
        };

        let entry_point_name = CString::new("main").unwrap();

        let stage_info = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(shader_module)
            .name(&entry_point_name);

        let pipeline_info = vk::ComputePipelineCreateInfo::default()
            .stage(stage_info)
            .layout(pipeline_layout);

        let preferred_flags = engine.driver_specializer.preferred_pipeline_flags();
        let pipeline_info = pipeline_info.flags(preferred_flags);

        let pipelines = unsafe {
            device
                .create_compute_pipelines(pipeline_cache, &[pipeline_info], None)
                .map_err(|(_, err)| {
                    anyhow::anyhow!(
                        "[ComputePipelineFactory] Failed to create Vulkan Compute Pipeline: {:?}",
                        err
                    )
                })?
        };

        unsafe {
            device.destroy_shader_module(shader_module, None);
        }

        let pipeline = pipelines[0];

        Ok((pipeline, pipeline_layout))
    }
}
