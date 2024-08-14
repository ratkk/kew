use crate::core::buffer::KewBuffer;
use crate::core::device::KewDevice;
use crate::core::memory::{KewMemory, KewMemoryBinding};
use ash::vk;
use log;
use std::ops::Deref;
use log::{debug, warn};

pub struct KewImage<'a> {
    kew_device: &'a KewDevice,
    m_bind: Option<KewMemoryBinding<'a>>,
    vk_image: vk::Image,
    format: vk::Format,
    pub b_size: vk::DeviceSize,
    pub extent: vk::Extent3D,
    pub layout: vk::ImageLayout,
    pub subresource: vk::ImageSubresourceRange,
    pub view: Option<vk::ImageView>,
}

impl<'a> KewImage<'a> {
    pub fn new(
        kew_device: &'a KewDevice,
        image_dx: u32,
        image_dy: u32,
        format: vk::Format,
        b_size: vk::DeviceSize,
        usage: vk::ImageUsageFlags,
    ) -> Self {
        let extent = vk::Extent3D::default()
            .width(image_dx)
            .depth(1)
            .height(image_dy);

        let create_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(extent)
            .mip_levels(1)
            .array_layers(1)
            .format(format)
            .tiling(vk::ImageTiling::OPTIMAL)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(usage)
            .samples(vk::SampleCountFlags::TYPE_1)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let vk_image = unsafe {
            kew_device
                .create_image(&create_info, None)
                .expect("failed to create image")
        };

        let subresource_range = vk::ImageSubresourceRange::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_mip_level(0)
            .level_count(1)
            .base_array_layer(0)
            .layer_count(1);

        Self {
            vk_image,
            kew_device,
            m_bind: None,
            b_size,
            extent,
            layout: vk::ImageLayout::UNDEFINED,
            format,
            subresource: subresource_range,
            view: None,
        }
    }

    pub fn recreate_image_view(&mut self) {
        let create_info = vk::ImageViewCreateInfo::default()
            .image(self.vk_image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(self.format)
            .subresource_range(self.subresource);
        self.view = unsafe {
            Some(
                self.kew_device
                    .create_image_view(&create_info, None)
                    .unwrap(),
            )
        };
    }

    pub fn bind_memory(&mut self, memory: &'a KewMemory, offset: vk::DeviceSize) {
        if self.m_bind.is_none() {
            self.m_bind = Some(KewMemoryBinding { memory, offset });
            unsafe {
                self.kew_device
                    .bind_image_memory(self.vk_image, memory.memory, offset)
                    .expect("failed to bind memory")
            };
        } else {
            warn!("image already bound to memory (skipped)")
        }
    }

    pub fn get_memory_barrier(
        &self,
        dst_layout: vk::ImageLayout,
        src_access_flags: vk::AccessFlags,
        dst_access_flags: vk::AccessFlags,
    ) -> vk::ImageMemoryBarrier {
        vk::ImageMemoryBarrier::default()
            .old_layout(self.layout)
            .new_layout(dst_layout)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(self.vk_image)
            .subresource_range(self.subresource)
            .src_access_mask(src_access_flags)
            .dst_access_mask(dst_access_flags)
    }

    pub fn copy_to_buffer(&self, buffer: &KewBuffer, cmd_buffer: vk::CommandBuffer) {
        let subresource_info = vk::ImageSubresourceLayers::default()
            .aspect_mask(self.subresource.aspect_mask)
            .mip_level(self.subresource.base_mip_level)
            .base_array_layer(self.subresource.base_array_layer)
            .layer_count(1);

        let copy_region = vk::BufferImageCopy::default()
            .buffer_offset(0)
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(subresource_info)
            .image_offset(vk::Offset3D::default().x(0).y(0).z(0))
            .image_extent(self.extent);

        unsafe {
            self.kew_device.cmd_copy_image_to_buffer(
                cmd_buffer,
                self.vk_image,
                self.layout,
                **buffer,
                std::slice::from_ref(&copy_region),
            );
        }
    }

    pub fn get_offset(&self) -> u64 {
        if let Some(binding) = &self.m_bind {
            binding.offset
        } else {
            panic!("cannot return offset for unbound image")
        }
    }

    // TODO: allow passing sampler for sharing between images
    pub fn descriptor_info(&mut self) -> vk::DescriptorImageInfo {
        let view = self.view.unwrap_or_else(|| {
            warn!("view missing for descriptor info (recreated view)");
            self.recreate_image_view();
            self.view.unwrap()
        });
        vk::DescriptorImageInfo::default()
            .image_layout(self.layout)
            .image_view(view)
            .sampler(vk::Sampler::default())
    }

    pub fn get_memory_requirements(&self) -> vk::MemoryRequirements {
        unsafe { self.kew_device.get_image_memory_requirements(self.vk_image) }
    }
}

impl Deref for KewImage<'_> {
    type Target = vk::Image;

    fn deref(&self) -> &Self::Target {
        &self.vk_image
    }
}

impl Drop for KewImage<'_> {
    fn drop(&mut self) {
        debug!("dropping KewImage");
        unsafe {
            self.kew_device.destroy_image(self.vk_image, None);
            if let Some(view) = self.view {
                self.kew_device.destroy_image_view(view, None);
            }
        }
    }
}
