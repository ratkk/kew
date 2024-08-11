use crate::core::buffer::KewBuffer;
use crate::math::vector::Vector;
use ash::vk;
use std::mem::size_of;

pub struct KewModel<'a> {
    buffer: KewBuffer<'a>,
    index_offset: usize,
}

pub struct KewModelVertexData<T: KewVertex> {
    vertices: Vec<T>,
    indices: Option<Vec<[u32; 3]>>,
}

impl KewModelVertexData<FlatVertex> {
    pub fn square() -> Self {
        let vertices = vec![
            FlatVertex::new([-0.8, 0.8], [1.0, 0.0, 0.0]),
            FlatVertex::new([-0.8, -0.8], [0.0, 0.0, 1.0]),
            FlatVertex::new([0.8, 0.8], [1.0, 0.0, 0.0]),
            FlatVertex::new([0.8, -0.8], [0.0, 0.0, 1.0]),
        ];
        let indices = Some(vec![[1, 2, 3], [0, 1, 2]]);
        Self { vertices, indices }
    }
}

pub enum VertexType {
    NULL,
    FLAT,
}

impl VertexType {
    pub const fn bind_descriptions(&self) -> Option<&'static [vk::VertexInputBindingDescription]> {
        match self {
            VertexType::NULL => None,
            VertexType::FLAT => Some(FlatVertex::bind_descriptions()),
        }
    }

    pub const fn attr_descriptions(
        &self,
    ) -> Option<&'static [vk::VertexInputAttributeDescription]> {
        match self {
            VertexType::NULL => None,
            VertexType::FLAT => Some(FlatVertex::attr_descriptions()),
        }
    }
}

pub trait KewVertex {}

pub struct FlatVertex {
    pos: Vector<f32, 2>,
    col: Vector<f32, 3>,
}

impl FlatVertex {
    pub fn new(pos: [f32; 2], col: [f32; 3]) -> Self {
        Self {
            pos: pos.into(),
            col: col.into(),
        }
    }

    const fn bind_descriptions() -> &'static [vk::VertexInputBindingDescription] {
        &[vk::VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<Self>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }]
    }

    const fn attr_descriptions() -> &'static [vk::VertexInputAttributeDescription] {
        &[
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: 0,
            },
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: size_of::<Vector<f32, 2>>() as u32,
            },
        ]
    }
}

impl KewVertex for FlatVertex {}

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
