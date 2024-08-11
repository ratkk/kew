use std::ffi::CStr;
use ash::vk;
use crate::core::model::VertexType;
use crate::core::pipeline::{ColorTarget, GfxPipelineConfig, PrimitiveState};
use crate::core::shader::ShaderStageConfig;

pub const VERT_SHADER_CONFIG: ShaderStageConfig<0> = unsafe {
    ShaderStageConfig {
        entry_name: CStr::from_bytes_with_nul_unchecked(b"main\0"),
        path: "./shader/kew.vert.spv",
        bindings: [],
        stage: vk::ShaderStageFlags::VERTEX,
        create_flags: vk::PipelineShaderStageCreateFlags::empty(),
    }
};
pub const FRAG_SHADER_CONFIG: ShaderStageConfig<0> = unsafe {
    ShaderStageConfig {
        entry_name: CStr::from_bytes_with_nul_unchecked(b"main\0"),
        path: "./shader/kew.frag.spv",
        bindings: [],
        stage: vk::ShaderStageFlags::FRAGMENT,
        create_flags: vk::PipelineShaderStageCreateFlags::empty(),
    }
};

pub const NULL_VERT_CONFIG: usize = 0;
pub const FLAT_VERT_CONFIG: usize = 1;

pub const PIPELINE_CONFIGS: [GfxPipelineConfig; 2] = [
    GfxPipelineConfig {
        primitive: PrimitiveState {
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            restart: false,
            polygon_mode: vk::PolygonMode::FILL,
            depth_clamp: false,
            cull_mode: vk::CullModeFlags::NONE,
            front_face: vk::FrontFace::CLOCKWISE,
        },
        color_targets: &[ColorTarget {
            color_blend: None,
            alpha_blend: None,
            write_mask: vk::ColorComponentFlags::RGBA,
        }],
        vertex_type: VertexType::NULL,
    },
    GfxPipelineConfig {
        primitive: PrimitiveState {
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            restart: false,
            polygon_mode: vk::PolygonMode::FILL,
            depth_clamp: false,
            cull_mode: vk::CullModeFlags::NONE,
            front_face: vk::FrontFace::CLOCKWISE,
        },
        color_targets: &[ColorTarget {
            color_blend: None,
            alpha_blend: None,
            write_mask: vk::ColorComponentFlags::RGBA,
        }],
        vertex_type: VertexType::FLAT,
    }
];