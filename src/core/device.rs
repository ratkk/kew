use crate::core::context::KewContext;
use ash::khr::{surface, swapchain};
use ash::{vk, Device};
use log::debug;
use std::ops::Deref;

pub struct KewDevice {
    pub context: KewContext,
    vk_device: Device,
}

impl KewDevice {
    pub fn new(context: KewContext, queue_indices: &KewQueueIndices) -> Self {
        let queue_create_infos = queue_indices.get_queue_create_infos();
        let device_features = vk::PhysicalDeviceFeatures::default();
        let extension_names = [swapchain::NAME.as_ptr()];

        let create_info = vk::DeviceCreateInfo::default()
            .enabled_features(&device_features)
            .enabled_extension_names(&extension_names)
            .queue_create_infos(&queue_create_infos);
        let vk_device = unsafe {
            context
                .instance
                .create_device(context.physical, &create_info, None)
                .unwrap_or_else(|e| panic!("failed to create logical device: {}", e))
        };
        Self { context, vk_device }
    }

    pub fn find_memory_type(
        &self,
        memory_reqs: &vk::MemoryRequirements,
        memory_flag: vk::MemoryPropertyFlags,
    ) -> Option<u32> {
        self.context.mem_properties.memory_types
            [..self.context.mem_properties.memory_type_count as _]
            .iter()
            .enumerate()
            .find(|(idx, memory_type)| {
                (1 << idx) as u32 & memory_reqs.memory_type_bits != 0
                    && memory_type.property_flags & memory_flag == memory_flag
            })
            .map(|(idx, _memory_type)| idx as _)
    }
}

impl Deref for KewDevice {
    type Target = Device;

    fn deref(&self) -> &Self::Target {
        &self.vk_device
    }
}

impl Drop for KewDevice {
    fn drop(&mut self) {
        debug!("dropping KewDevice");
        unsafe {
            self.vk_device.destroy_device(None);
        }
    }
}

pub struct KewQueueIndices {
    pub gfx_idx: u32,
    pub cmp_idx: u32,
    pub tfr_idx: u32,
    pub prs_idx: u32,
}

impl KewQueueIndices {
    // TODO: add heuristics
    pub fn new(
        context: &KewContext,
        surface_loader: &surface::Instance,
        surface: vk::SurfaceKHR,
    ) -> Self {
        let queue_families = unsafe {
            context
                .instance
                .get_physical_device_queue_family_properties(context.physical)
        };
        let mut gfx_idx: (Option<u32>, bool) = (None, false);
        let mut cmp_idx: (Option<u32>, bool) = (None, false);
        let mut tfr_idx: (Option<u32>, bool) = (None, false);
        let mut prs_idx: Option<u32> = None;

        for (idx, qfp) in queue_families.iter().enumerate() {
            let idx = idx as u32;
            if gfx_idx.0.is_none() && qfp.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                gfx_idx.0 = Some(idx);
            }
            if cmp_idx.0.is_none() && qfp.queue_flags.contains(vk::QueueFlags::COMPUTE) {
                cmp_idx.0 = Some(idx);
            }
            if tfr_idx.0.is_none() && qfp.queue_flags.contains(vk::QueueFlags::TRANSFER) {
                tfr_idx.0 = Some(idx);
            }
            let present_support = unsafe {
                surface_loader
                    .get_physical_device_surface_support(context.physical, idx, surface)
                    .unwrap()
            };
            if prs_idx.is_none() && present_support {
                prs_idx = Some(idx);
            }

            // prefer non-graphics compute queue
            if cmp_idx.0.is_some()
                && !cmp_idx.1
                && qfp.queue_flags.contains(vk::QueueFlags::COMPUTE)
                && !qfp.queue_flags.contains(vk::QueueFlags::GRAPHICS)
            {
                cmp_idx.0 = Some(idx);
                cmp_idx.1 = true;
                continue;
            }

            // prefer non-graphics and non-compute transfer queue
            if tfr_idx.0.is_some()
                && !tfr_idx.1
                && qfp.queue_flags.contains(vk::QueueFlags::TRANSFER)
                && !qfp
                    .queue_flags
                    .contains(vk::QueueFlags::GRAPHICS | vk::QueueFlags::COMPUTE)
            {
                tfr_idx.0 = Some(idx);
                tfr_idx.1 = true;
                continue;
            }
        }
        match (gfx_idx.0, cmp_idx.0, tfr_idx.0, prs_idx) {
            (Some(gfx), Some(cmp), Some(tfr), Some(prs)) => Self {
                gfx_idx: gfx,
                cmp_idx: cmp,
                tfr_idx: tfr,
                prs_idx: prs,
            },
            _ => panic!("failed to find required queue families"),
        }
    }

    fn get_queue_create_infos(&self) -> Vec<vk::DeviceQueueCreateInfo> {
        let mut indices = vec![self.gfx_idx, self.cmp_idx, self.tfr_idx];
        indices.dedup();
        indices
            .iter()
            .map(|idx| {
                vk::DeviceQueueCreateInfo::default()
                    .queue_family_index(*idx)
                    .queue_priorities(&[1.0f32])
            })
            .collect::<Vec<_>>()
    }
}
