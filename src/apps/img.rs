use std::ffi::CStr;
use std::path::Path;

use ash::vk;
use ash::vk::{CommandBuffer, ImageMemoryBarrier};
use image::{open, RgbaImage};

use crate::core::buffer::KewBuffer;
use crate::core::command::KewCommandPool;
use crate::core::descriptor::KewDescriptorPoolBuilder;
use crate::core::device::KewDevice;
use crate::core::image::KewImage;
use crate::core::memory::KewMemory;
use crate::core::pipeline::KewCmpPipeline;
use crate::core::shader::{DescriptorSetLayoutBindingInfo, KewShader, ShaderStageConfig};

const IMG_SHADER_CONFIG: ShaderStageConfig<2> = unsafe {
    ShaderStageConfig {
        entry_name: CStr::from_bytes_with_nul_unchecked(b"main\0"),
        path: "./shader/img.spv",
        bindings: [
            DescriptorSetLayoutBindingInfo {
                descriptor_type: vk::DescriptorType::STORAGE_IMAGE,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::COMPUTE,
            },
            DescriptorSetLayoutBindingInfo {
                descriptor_type: vk::DescriptorType::STORAGE_IMAGE,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::COMPUTE,
            },
        ],
        stage: vk::ShaderStageFlags::COMPUTE,
        create_flags: vk::PipelineShaderStageCreateFlags::empty(),
    }
};
pub fn img_compute(
    kew_device: &KewDevice,
    cmp_cmd_pool: &KewCommandPool,
    tfr_cmd_pool: &KewCommandPool,
) {
    let image = open("assets/mcry.jpg").unwrap().into_rgba8();
    let (img_dx, img_dy) = image.dimensions();
    let b_size_img: u64 = (img_dx * img_dy * 4) as u64;

    let stage_memory: KewMemory;
    let mut stage_buffer = KewBuffer::new(
        kew_device,
        b_size_img,
        vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST,
    );
    stage_memory = KewMemory::new(
        kew_device,
        stage_buffer.get_memory_requirements().size,
        stage_buffer.get_memory_requirements(),
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    );
    stage_buffer.bind_memory(&stage_memory, 0);
    unsafe {
        stage_memory.map(b_size_img, 0);
        stage_memory.wr_visible_mem(image.as_raw(), b_size_img, 0);
    }

    let (src_img_memory, mut src_img) = image_malloc(
        kew_device,
        img_dx,
        img_dy,
        vk::ImageUsageFlags::TRANSFER_DST
            | vk::ImageUsageFlags::SAMPLED
            | vk::ImageUsageFlags::STORAGE,
    );
    let (dst_img_memory, mut dst_img) = image_malloc(
        kew_device,
        img_dx,
        img_dy,
        vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::STORAGE,
    );
    src_img.bind_memory(&src_img_memory, 0);
    dst_img.bind_memory(&dst_img_memory, 0);

    let b0 = src_img.get_memory_barrier(
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        vk::AccessFlags::NONE,
        vk::AccessFlags::TRANSFER_WRITE,
    );
    let cmd_buffer = tfr_cmd_pool.get_command_buffer();
    unsafe {
        kew_device.cmd_pipeline_barrier(
            cmd_buffer,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::TRANSFER,
            vk::DependencyFlags::BY_REGION,
            &[],
            &[],
            &[b0],
        );
        tfr_cmd_pool.submit_command_buffers(&[cmd_buffer]).unwrap();
        src_img.layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
    }
    stage_buffer.copy_to_image(&src_img, tfr_cmd_pool);

    let b0 = src_img.get_memory_barrier(
        vk::ImageLayout::GENERAL,
        vk::AccessFlags::TRANSFER_WRITE,
        vk::AccessFlags::SHADER_READ,
    );
    let b1 = dst_img.get_memory_barrier(
        vk::ImageLayout::GENERAL,
        vk::AccessFlags::NONE,
        vk::AccessFlags::SHADER_WRITE,
    );
    let src_img_cmd_buffer = cmp_cmd_pool.get_command_buffer();
    let dst_img_cmd_buffer = cmp_cmd_pool.get_command_buffer();
    unsafe {
        image_layout_transition(
            kew_device,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            b0,
            src_img_cmd_buffer,
        );
        image_layout_transition(
            kew_device,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            b1,
            dst_img_cmd_buffer,
        );
        cmp_cmd_pool
            .submit_command_buffers(&[src_img_cmd_buffer, dst_img_cmd_buffer])
            .unwrap();
        src_img.layout = vk::ImageLayout::GENERAL;
        dst_img.layout = vk::ImageLayout::GENERAL
    }

    let shader = KewShader::new(kew_device, &IMG_SHADER_CONFIG);
    let descriptor_pool = KewDescriptorPoolBuilder::new(1)
        .add_pool_size(vk::DescriptorType::STORAGE_IMAGE, 2)
        .build(kew_device);

    let set = unsafe { descriptor_pool.allocate_descriptor_set(shader.descriptor_set_layout) };
    shader.write_image(0, src_img.descriptor_info(), &set);
    shader.write_image(1, dst_img.descriptor_info(), &set);
    let pipeline = KewCmpPipeline::new(kew_device, &shader);

    let command_buffer = pipeline.get_bound_cmd_buffer(cmp_cmd_pool, set);
    unsafe {
        kew_device.cmd_dispatch(command_buffer, img_dx, img_dy, 1);
        cmp_cmd_pool
            .submit_command_buffers(&[command_buffer])
            .unwrap()
    }
    dst_img.copy_to_buffer(&stage_buffer, tfr_cmd_pool);

    let mut result = RgbaImage::new(img_dx, img_dy);
    unsafe {
        stage_memory.rd_visible_mem(&mut result, b_size_img, dst_img.get_offset());
    }
    result.save(Path::new("./assets/result.png")).unwrap()
}

unsafe fn image_layout_transition(
    kew_device: &KewDevice,
    src_pipeline_stage: vk::PipelineStageFlags,
    dst_pipeline_stage: vk::PipelineStageFlags,
    memory_barrier: ImageMemoryBarrier,
    cmd_buffer: CommandBuffer,
) {
    kew_device.cmd_pipeline_barrier(
        cmd_buffer,
        src_pipeline_stage,
        dst_pipeline_stage,
        vk::DependencyFlags::BY_REGION,
        &[],
        &[],
        &[memory_barrier],
    );
}

fn image_malloc<'a>(
    kew_device: &'a KewDevice,
    image_dx: u32,
    image_dy: u32,
    usage_flags: vk::ImageUsageFlags,
) -> (KewMemory, KewImage<'a>) {
    let image_memory: KewMemory;
    let image_b_size = (image_dx * image_dy * 4) as u64;
    let image = KewImage::new(
        kew_device,
        image_dx,
        image_dy,
        vk::Format::R8G8B8A8_UNORM,
        image_b_size,
        usage_flags,
    );
    image_memory = KewMemory::new(
        kew_device,
        image.get_memory_requirements().size,
        image.get_memory_requirements(),
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    );
    (image_memory, image)
}
