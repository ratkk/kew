use crate::core::device::KewDevice;
use ash::prelude::VkResult;
use ash::vk;
use ash::vk::CommandPool;
use log::debug;

pub struct KewCommandPool<'a> {
    kew_device: &'a KewDevice,
    pub queue: vk::Queue,
    command_pool: CommandPool,
}

impl<'a> KewCommandPool<'a> {
    pub fn new(kew_device: &'a KewDevice, queue_idx: u32) -> Self {
        let create_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(queue_idx)
            .flags(
                vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER
                    | vk::CommandPoolCreateFlags::TRANSIENT,
            );
        unsafe {
            let command_pool = kew_device
                .create_command_pool(&create_info, None)
                .expect("failed to create command pool");
            let queue = kew_device.get_device_queue(queue_idx, 0);
            Self {
                kew_device,
                queue,
                command_pool,
            }
        }
    }

    pub fn create_command_buffers(
        &self,
        level: vk::CommandBufferLevel,
        count: u32,
    ) -> Vec<vk::CommandBuffer> {
        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .level(level)
            .command_pool(self.command_pool)
            .command_buffer_count(count);
        unsafe {
            self.kew_device
                .allocate_command_buffers(&alloc_info)
                .expect("failed allocating command buffers")
        }
    }

    pub fn get_command_buffer(&self) -> vk::CommandBuffer {
        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_pool(self.command_pool)
            .command_buffer_count(1);
        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe {
            let command_buffer = self
                .kew_device
                .allocate_command_buffers(&alloc_info)
                .expect("failed to allocate command buffer")[0];
            self.kew_device
                .begin_command_buffer(command_buffer, &begin_info)
                .expect("failed to begin command buffer");
            command_buffer
        }
    }

    pub unsafe fn submit_command_buffers(
        &self,
        command_buffers: &[vk::CommandBuffer],
    ) -> VkResult<()> {
        command_buffers.iter().for_each(|command_buffer| {
            self.kew_device.end_command_buffer(*command_buffer).unwrap();
        });
        let submit_info = vk::SubmitInfo::default().command_buffers(command_buffers);
        self.kew_device.queue_submit(
            self.queue,
            std::slice::from_ref(&submit_info),
            vk::Fence::null(),
        )?;
        self.kew_device.queue_wait_idle(self.queue)?;
        self.kew_device
            .free_command_buffers(self.command_pool, command_buffers);
        Ok(())
    }
}

impl Drop for KewCommandPool<'_> {
    fn drop(&mut self) {
        unsafe {
            debug!("dropping KewCommandPool");
            self.kew_device
                .destroy_command_pool(self.command_pool, None);
        }
    }
}
