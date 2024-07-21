use crate::math::vector::Vector;
use ash::vk;
use std::mem::size_of;

pub struct Vertex {
    pub position: Vector<f32, 3>,
    pub normal: Vector<f32, 3>,
    pub color: Vector<f32, 3>,
    pub texture: Vector<f32, 2>,
}

impl Vertex {
    pub fn bind_descriptions() -> Vec<vk::VertexInputBindingDescription> {
        let vertex_size = size_of::<Vertex>() as u32;

        vec![vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(vertex_size)
            .input_rate(vk::VertexInputRate::VERTEX)]
    }

    pub fn attr_descriptions() -> Vec<vk::VertexInputAttributeDescription> {
        let vector3_size = size_of::<Vector<f32, 3>>() as u32;
        vec![
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: 0,
            },
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: vector3_size,
            },
            vk::VertexInputAttributeDescription {
                location: 2,
                binding: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: 2 * vector3_size,
            },
            vk::VertexInputAttributeDescription {
                location: 3,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: 3 * vector3_size,
            },
        ]
    }
}
