use crate::profile::HardwareProfile;
use anyhow::Result;
use ash::vk;
use std::sync::Arc;

pub trait DriverSpecializer: Send + Sync {
    fn specialize_and_optimize(
        &self,
        spirv_code: &[u8],
        _profile: &HardwareProfile,
    ) -> Result<Vec<u8>> {
        Ok(spirv_code.to_vec())
    }

    fn preferred_pipeline_flags(&self) -> vk::PipelineCreateFlags {
        vk::PipelineCreateFlags::empty()
    }

    fn query_jit_defines(&self, _profile: &HardwareProfile) -> Vec<(String, String)> {
        Vec::new()
    }
}

#[derive(Default)]
pub struct GenericDriver;
impl DriverSpecializer for GenericDriver {}

#[derive(Default)]
pub struct NvidiaDriver;
impl DriverSpecializer for NvidiaDriver {}

#[derive(Default)]
pub struct AmdDriver;
impl DriverSpecializer for AmdDriver {}

#[derive(Default)]
pub struct IntelDriver;
impl DriverSpecializer for IntelDriver {}

pub fn select_driver(vendor_id: u32) -> Arc<dyn DriverSpecializer> {
    match vendor_id {
        0x10DE => Arc::new(NvidiaDriver),
        0x1002 | 0x1022 => Arc::new(AmdDriver),
        0x8086 => Arc::new(IntelDriver),
        _ => Arc::new(GenericDriver),
    }
}
