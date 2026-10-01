use anyhow::{Context, Result};
use ash::vk;
use serde::{Deserialize, Serialize};
use std::ffi::CStr;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CooperativeMatrixSpec {
    pub m_size: u32,
    pub n_size: u32,
    pub k_size: u32,
    pub a_type: i32,
    pub b_type: i32,
    pub c_type: i32,
    pub result_type: i32,
    pub scope: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareProfile {
    pub vendor_id: u32,
    pub device_id: u32,
    pub device_name: String,
    pub is_integrated: bool,

    pub subgroup_size: u32,
    pub subgroup_supported_stages: u32,
    pub subgroup_supported_operations: u32,
    pub quad_operations_in_all_stages: bool,

    pub supports_cooperative_matrix: bool,
    pub cooperative_matrix_supported_stages: u32,
    pub supported_matrices: Vec<CooperativeMatrixSpec>,

    pub max_compute_workgroup_invocations: u32,
    pub max_compute_shared_memory_size: u32,
    pub max_push_constants_size: u32,
}

pub struct ProfileQuerier;

impl ProfileQuerier {
    pub fn query_profile(
        entry: &ash::Entry,
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
    ) -> Result<HardwareProfile> {
        let properties = unsafe { instance.get_physical_device_properties(physical_device) };
        let device_name = unsafe {
            CStr::from_ptr(properties.device_name.as_ptr())
                .to_string_lossy()
                .into_owned()
        };

        let extension_properties = unsafe {
            instance
                .enumerate_device_extension_properties(physical_device)
                .context("[ProfileQuerier] Failed to enumerate device extensions")?
        };

        let is_integrated = properties.device_type == vk::PhysicalDeviceType::INTEGRATED_GPU;

        let supports_cooperative_matrix = extension_properties.iter().any(|ext| {
            let name = unsafe { CStr::from_ptr(ext.extension_name.as_ptr()) };
            name.to_str() == Ok("VK_KHR_cooperative_matrix")
        });

        let mut coop_matrix_properties =
            vk::PhysicalDeviceCooperativeMatrixPropertiesKHR::default();

        let mut subgroup_properties = vk::PhysicalDeviceSubgroupProperties {
            p_next: if supports_cooperative_matrix {
                &mut coop_matrix_properties as *mut _ as *mut _
            } else {
                std::ptr::null_mut()
            },
            ..Default::default()
        };

        let mut properties2 = vk::PhysicalDeviceProperties2 {
            p_next: &mut subgroup_properties as *mut _ as *mut _,
            ..Default::default()
        };

        unsafe {
            instance.get_physical_device_properties2(physical_device, &mut properties2);
        }

        let mut supported_matrices = Vec::new();
        if supports_cooperative_matrix {
            let coop_matrix_ext = ash::khr::cooperative_matrix::Instance::new(entry, instance);

            if let Ok(matrices) = unsafe {
                coop_matrix_ext.get_physical_device_cooperative_matrix_properties(physical_device)
            } {
                supported_matrices = matrices
                    .iter()
                    .map(|m| CooperativeMatrixSpec {
                        m_size: m.m_size,
                        n_size: m.n_size,
                        k_size: m.k_size,
                        a_type: m.a_type.as_raw(),
                        b_type: m.b_type.as_raw(),
                        c_type: m.c_type.as_raw(),
                        result_type: m.result_type.as_raw(),
                        scope: m.scope.as_raw(),
                    })
                    .collect();
            }
        }

        Ok(HardwareProfile {
            vendor_id: properties.vendor_id,
            device_id: properties.device_id,
            device_name,
            is_integrated,

            subgroup_size: subgroup_properties.subgroup_size,
            subgroup_supported_stages: subgroup_properties.supported_stages.as_raw(),
            subgroup_supported_operations: subgroup_properties.supported_operations.as_raw(),
            quad_operations_in_all_stages: subgroup_properties.quad_operations_in_all_stages != 0,

            supports_cooperative_matrix,
            cooperative_matrix_supported_stages: coop_matrix_properties
                .cooperative_matrix_supported_stages
                .as_raw(),
            supported_matrices,

            max_compute_workgroup_invocations: properties.limits.max_compute_work_group_invocations,
            max_compute_shared_memory_size: properties.limits.max_compute_shared_memory_size,
            max_push_constants_size: properties.limits.max_push_constants_size,
        })
    }
}
