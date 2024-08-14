use crate::core::buffer::KewBuffer;
use crate::core::device::KewDevice;
use crate::math::vector::Vector;
use ash::vk;
use std::mem::size_of;

pub struct KewModel {
    pub vertex_offset: u32,
    pub index_amount: u32,
    pub index_offset: u32,
}

impl KewModel {
    pub unsafe fn bind(
        &self,
        kew_device: &KewDevice,
        cmd_buffer: vk::CommandBuffer,
        vrt_buffer: &KewBuffer,
        idx_buffer: &KewBuffer,
    ) {
        let buffers = [vrt_buffer.vk_buffer];
        let offsets = [self.vertex_offset as u64];
        kew_device.cmd_bind_vertex_buffers(cmd_buffer, 0, &buffers, &offsets);
        kew_device.cmd_bind_index_buffer(
            cmd_buffer,
            idx_buffer.vk_buffer,
            self.index_offset as u64,
            vk::IndexType::UINT32,
        );
    }

    pub unsafe fn draw(&self, kew_device: &KewDevice, cmd_buffer: vk::CommandBuffer) {
        kew_device.cmd_draw_indexed(cmd_buffer, self.index_amount, 1, 0, 0, 0);
    }
}

pub struct KewModelVertexData<T: KewVertex + Clone + Copy> {
    pub vertices: Vec<T>,
    pub indices: Vec<u32>,
}

impl<T: KewVertex + Clone + Copy> KewModelVertexData<T> {
    pub fn vertex_data_size(&self) -> u64 {
        self.vertices.len() as u64 * T::vertex_size()
    }

    pub fn index_data_size(&self) -> u64 {
        self.indices.len() as u64 * 4
    }

    pub unsafe fn write_to_memory(
        &self,
        vrt_buffer: &KewBuffer,
        idx_buffer: &KewBuffer,
        vrt_offset: u64,
        idx_offset: u64,
    ) {
        vrt_buffer.wr_visible_mem(&self.vertices, self.vertex_data_size(), vrt_offset);
        idx_buffer.wr_visible_mem(&self.indices, self.index_data_size(), idx_offset);
    }
}

impl KewModelVertexData<FlatVertex> {
    pub fn square() -> Self {
        let vertices = vec![
            FlatVertex::new([-0.8, 0.8], [1.0, 0.0, 0.0]),
            FlatVertex::new([-0.8, -0.8], [0.0, 0.0, 1.0]),
            FlatVertex::new([0.8, 0.8], [1.0, 0.0, 0.0]),
            FlatVertex::new([0.8, -0.8], [0.0, 0.0, 1.0]),
        ];
        Self {
            vertices,
            indices: vec![0, 1, 2, 2, 1, 3],
        }
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

pub trait KewVertex: Sized {
    fn vertex_size() -> u64 {
        size_of::<Self>() as u64
    }
}

#[derive(Clone, Copy)]
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
