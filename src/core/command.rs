use crate::core::device::KewDevice;
use ash::prelude::VkResult;
use ash::vk;
use ash::vk::CommandPool;
use log::debug;
use std::mem::MaybeUninit;

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

    pub fn allocate_command_buffers<const N: usize>(
        &self,
        level: vk::CommandBufferLevel,
    ) -> [vk::CommandBuffer; N] {
        let mut cmd_buffers: [MaybeUninit<vk::CommandBuffer>; N] =
            [const { MaybeUninit::uninit() }; N];
        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .level(level)
            .command_pool(self.command_pool)
            .command_buffer_count(N as u32);
        unsafe {
            let cmd_buffers_vec = self
                .kew_device
                .allocate_command_buffers(&alloc_info)
                .expect("failed allocating command buffers");
            for (index, cmd_buffer) in cmd_buffers_vec.into_iter().enumerate().take(N) {
                cmd_buffers[index].write(cmd_buffer.to_owned());
            }
            std::ptr::read(cmd_buffers.as_ptr() as *const [vk::CommandBuffer; N])        }
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
