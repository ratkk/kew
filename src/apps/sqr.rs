use crate::core::buffer::KewBuffer;
use crate::core::command::KewCommandPool;
use crate::core::descriptor::KewDescriptorPoolBuilder;
use crate::core::device::KewDevice;
use crate::core::memory::KewMemory;
use crate::core::pipeline::KewCmpPipeline;
use crate::core::shader::{DescriptorSetLayoutBindingInfo, KewShader, ShaderStageConfig};
use ash::vk;
use std::ffi::CStr;

const SQR_SHADER_CONFIG: ShaderStageConfig<2> = unsafe {
    ShaderStageConfig {
        entry_name: CStr::from_bytes_with_nul_unchecked(b"main\0"),
        path: "./shader/sqr.spv",
        bindings: [
            DescriptorSetLayoutBindingInfo {
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::COMPUTE,
            },
            DescriptorSetLayoutBindingInfo {
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::COMPUTE,
            },
        ],
        stage: vk::ShaderStageFlags::COMPUTE,
        create_flags: vk::PipelineShaderStageCreateFlags::empty(),
    }
};

pub fn sqr_compute(kew_device: &KewDevice, cmp_cmd_pool: &KewCommandPool, data: &[i32]) {
    let buffer_b_size = (data.len() * 4) as u64;

    let buffer_memory: KewMemory;
    let mut src_buffer = KewBuffer::new(
        kew_device,
        buffer_b_size,
        vk::BufferUsageFlags::STORAGE_BUFFER,
    );
    let mut dst_buffer = KewBuffer::new(
        kew_device,
        buffer_b_size,
        vk::BufferUsageFlags::STORAGE_BUFFER,
    );

    let min_alignment = src_buffer.get_memory_requirements().alignment;
    let memory_b_size = (buffer_b_size + min_alignment - 1) & !(min_alignment - 1);

    buffer_memory = KewMemory::new(
        kew_device,
        memory_b_size * 2,
        src_buffer.get_memory_requirements(),
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    );
    src_buffer.bind_memory(&buffer_memory, 0);
    dst_buffer.bind_memory(&buffer_memory, memory_b_size);
    unsafe {
        buffer_memory.map(vk::WHOLE_SIZE, 0);
        buffer_memory.wr_visible_mem(&data, buffer_b_size, 0);
    }

    let shader = KewShader::new(kew_device, &SQR_SHADER_CONFIG);
    let descriptor_pool = KewDescriptorPoolBuilder::new(1)
        .add_pool_size(vk::DescriptorType::STORAGE_BUFFER, 2)
        .build(kew_device);

    let set = unsafe { descriptor_pool.allocate_descriptor_set(shader.descriptor_set_layout) };
    shader.write_buffer(0, src_buffer.descriptor_info(), &set);
    shader.write_buffer(1, dst_buffer.descriptor_info(), &set);
    let pipeline = KewCmpPipeline::new(kew_device, &shader);

    let command_buffer = pipeline.get_bound_cmd_buffer(cmp_cmd_pool, set);
    unsafe {
        kew_device.cmd_dispatch(command_buffer, data.len() as u32, 1, 1);
        cmp_cmd_pool
            .submit_command_buffers(&[command_buffer])
            .unwrap()
    }
    let mut result: Vec<i32> = vec![0; data.len()];
    unsafe {
        buffer_memory.rd_visible_mem(&mut result, buffer_b_size, dst_buffer.get_offset());
    }
    log::info!("results: {:?}", result);
}
