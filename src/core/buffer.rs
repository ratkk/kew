use crate::core::device::KewDevice;
use crate::core::image::KewImage;
use crate::core::memory::{KewMemory, KewMemoryBinding};
use ash::vk;
use log::{debug, warn};
use std::ops::Deref;

pub struct KewBuffer<'a> {
    kew_device: &'a KewDevice,
    m_bind: Option<KewMemoryBinding<'a>>,
    pub vk_buffer: vk::Buffer,
    pub b_size: vk::DeviceSize,
}

impl<'a> KewBuffer<'a> {
    pub fn new(kew_device: &'a KewDevice, b_size: u64, usage: vk::BufferUsageFlags) -> Self {
        let create_info = vk::BufferCreateInfo::default()
            .size(b_size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let vk_buffer = unsafe {
            kew_device
                .create_buffer(&create_info, None)
                .expect("failed to create buffer")
        };
        Self {
            kew_device,
            m_bind: None,
            vk_buffer,
            b_size,
        }
    }

    pub fn bind_memory(&mut self, memory: &'a KewMemory, offset: vk::DeviceSize) {
        if self.m_bind.is_none() {
            self.m_bind = Some(KewMemoryBinding { memory, offset });
            unsafe {
                self.kew_device
                    .bind_buffer_memory(self.vk_buffer, memory.memory, offset)
                    .expect("failed to bind memory")
            };
        } else {
            warn!("buffer already bound to memory (skipped)")
        }
    }

    pub fn copy_to_image(&self, image: &KewImage, cmd_buffer: vk::CommandBuffer) {
        let subresource_info = vk::ImageSubresourceLayers::default()
            .aspect_mask(image.subresource.aspect_mask)
            .mip_level(image.subresource.base_mip_level)
            .base_array_layer(image.subresource.base_array_layer)
            .layer_count(1);

        let copy_region = vk::BufferImageCopy::default()
            .buffer_offset(0)
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(subresource_info)
            .image_offset(vk::Offset3D::default().x(0).y(0).z(0))
            .image_extent(image.extent);
        unsafe {
            self.kew_device.cmd_copy_buffer_to_image(
                cmd_buffer,
                self.vk_buffer,
                **image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                std::slice::from_ref(&copy_region),
            );
        }
    }

    pub unsafe fn wr_visible_mem<T: Copy>(
        &self,
        data: &[T],
        b_size: vk::DeviceSize,
        offset: vk::DeviceSize,
    ) {
        if let Some(binding) = &self.m_bind {
            binding
                .memory
                .wr_visible_mem(data, b_size, binding.offset + offset);
        } else {
            panic!("buffer not bound on write")
        }
    }

    pub fn descriptor_info(&self) -> vk::DescriptorBufferInfo {
        vk::DescriptorBufferInfo::default()
            .buffer(self.vk_buffer)
            .offset(0)
            .range(self.b_size)
    }

    pub fn get_memory_requirements(&self) -> vk::MemoryRequirements {
        unsafe {
            self.kew_device
                .get_buffer_memory_requirements(self.vk_buffer)
        }
    }
}

impl Deref for KewBuffer<'_> {
    type Target = vk::Buffer;

    fn deref(&self) -> &Self::Target {
        &self.vk_buffer
    }
}

impl Drop for KewBuffer<'_> {
    fn drop(&mut self) {
        debug!("dropping KewBuffer");
        unsafe {
            self.kew_device.destroy_buffer(self.vk_buffer, None);
        }
    }
}
