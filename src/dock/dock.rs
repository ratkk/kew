use crate::core::buffer::KewBuffer;
use crate::core::command::KewCommandPool;
use crate::core::descriptor::{KewDescriptorPool, KewDescriptorPoolBuilder};
use crate::core::device::{KewDevice, KewQueueIndices};
use crate::core::memory::KewMemory;
use crate::core::model::{KewModel, KewModelVertexData};
use crate::core::pipeline::KewGfxPipeline;
use crate::core::shader::KewShader;
use crate::core::swapchain::{KewSwapchain, MAX_IN_FLIGHT_FRAMES};
use crate::dock::config::{FLAT_VERT_CONFIG, FRAG_SHADER_CONFIG, PIPELINE_CONFIGS, VERT_SHADER_CONFIG};
use crate::dock::{DockErr, DockMessage};
use ash::khr::surface;
use ash::vk;
use crossbeam::channel::Receiver;
use log::{debug, error, warn};
use std::thread;

const MODEL_MEM_IDX: usize = 0;
const MODEL_MEM_SIZE: u64 = 1024;

pub fn init_dock(
    kew_device: &KewDevice,
    surface_loader: &surface::Instance,
    surface: vk::SurfaceKHR,
    queue_indices: &KewQueueIndices,
    window_extent: vk::Extent2D,
    application_thread: Receiver<DockMessage>,
) {
    let mut renderer = DockRenderer::new(
        &kew_device,
        &surface_loader,
        surface,
        window_extent,
        queue_indices.prs_idx,
        queue_indices.gfx_idx,
    );

    let mut memory_allocations: Vec<KewMemory> = Vec::with_capacity(8);

    let (mut vrt_buffer, vrt_mem_type) = create_buffer(
        kew_device,
        vk::BufferUsageFlags::VERTEX_BUFFER,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        MODEL_MEM_SIZE
    );
    let (mut idx_buffer, idx_mem_type) = create_buffer(
        kew_device,
        vk::BufferUsageFlags::INDEX_BUFFER,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        MODEL_MEM_SIZE
    );
    assert_eq!(vrt_mem_type, idx_mem_type);
    memory_allocations.push(KewMemory::new(
        kew_device,
        MODEL_MEM_SIZE * 2,
        vrt_mem_type,
    ));
    unsafe {
        memory_allocations
            .get(MODEL_MEM_IDX)
            .unwrap()
            .map(vk::WHOLE_SIZE, 0);
        vrt_buffer.bind_memory(&memory_allocations[MODEL_MEM_IDX], 0);
        idx_buffer.bind_memory(&memory_allocations[MODEL_MEM_IDX], MODEL_MEM_SIZE);
    }

    let vert_shader = KewShader::new(&kew_device, &VERT_SHADER_CONFIG);
    let frag_shader = KewShader::new(&kew_device, &FRAG_SHADER_CONFIG);

    thread::scope(|scope| {
        scope.spawn(|| {
            let mut dock_scene = DockScene::dummy(
                &kew_device,
                &vrt_buffer,
                &idx_buffer,
                &vert_shader,
                &frag_shader,
                &renderer.swapchain.render_pass,
            );

            let model = load_model(&vrt_buffer, &idx_buffer);
            dock_scene.add_model(model);

            loop {
                if let Ok(_) = application_thread.recv() {
                    renderer.render_scene(&dock_scene);
                } else {
                    error!("dock render thread error mpsc message received (dropping thread)");
                    unsafe {
                        surface_loader.destroy_surface(surface, None);
                        return;
                    }
                }
            }
        });
    });
}

fn create_buffer(
    kew_device: &KewDevice,
    buffer_usage: vk::BufferUsageFlags,
    memory_flags: vk::MemoryPropertyFlags,
    b_size: u64,
) -> (KewBuffer, u32) {
    let buffer = KewBuffer::new(
        kew_device,
        b_size,
        buffer_usage,
    );
    let memory_type = kew_device
        .find_memory_type(
            &buffer.get_memory_requirements(),
            memory_flags,
        )
        .unwrap();
    (buffer, memory_type)
}

fn load_model(vrt_buffer: &KewBuffer, idx_buffer: &KewBuffer) -> KewModel {
    let model_data = KewModelVertexData::square();
    unsafe {
        model_data.write_to_memory(
            vrt_buffer,
            idx_buffer,
            0,
            0
        );
    }
    KewModel {
        vertex_offset: 0,
        index_amount: model_data.indices.len() as u32,
        index_offset: 0,
    }
}

pub struct DockRenderer<'a> {
    kew_device: &'a KewDevice,
    swapchain: KewSwapchain<'a>,
    cmd_pool: KewCommandPool<'a>,
    cmd_buffers: [vk::CommandBuffer; MAX_IN_FLIGHT_FRAMES],
    descriptor_pool: KewDescriptorPool<'a>,
    current_frame_idx: usize,
    current_image_idx: usize,
    frame_opened: bool,
}

impl<'a> DockRenderer<'a> {
    pub fn new(
        kew_device: &'a KewDevice,
        surface_loader: &surface::Instance,
        surface: vk::SurfaceKHR,
        window_extent: vk::Extent2D,
        prs_queue_idx: u32,
        gfx_queue_idx: u32,
    ) -> Self {
        let cmd_pool = KewCommandPool::new(&kew_device, gfx_queue_idx);
        let cmd_buffers = cmd_pool
            .allocate_command_buffers::<MAX_IN_FLIGHT_FRAMES>(vk::CommandBufferLevel::PRIMARY);

        let swapchain = KewSwapchain::new(
            &kew_device,
            &surface_loader,
            surface,
            window_extent,
            prs_queue_idx,
        );
        let descriptor_pool = KewDescriptorPoolBuilder::new(MAX_IN_FLIGHT_FRAMES as u32)
            .add_pool_size(
                vk::DescriptorType::UNIFORM_BUFFER,
                MAX_IN_FLIGHT_FRAMES as u32,
            )
            .build(kew_device);

        Self {
            kew_device,
            swapchain,
            cmd_pool,
            cmd_buffers,
            descriptor_pool,
            current_frame_idx: 0,
            current_image_idx: 0,
            frame_opened: false,
        }
    }

    pub fn render_scene(&mut self, scene: &DockScene) {
        unsafe {
            if let Ok(cmd_buffer) = self.open_frame() {
                self.swapchain
                    .begin_render_pass(cmd_buffer, self.current_image_idx);
                scene.record_cmd_buffer(cmd_buffer);
                self.swapchain.end_render_pass(cmd_buffer);
                self.close_frame(cmd_buffer);
            }
        }
    }

    unsafe fn open_frame(&mut self) -> Result<vk::CommandBuffer, DockErr> {
        if self.frame_opened || self.swapchain.frame_in_use(self.current_frame_idx) {
            debug!("dropped frame");
            return Err(DockErr::SOFT);
        }
        let image_idx_result = self.swapchain.next_image_idx(self.current_frame_idx);
        match image_idx_result {
            Err(_) => todo!(),
            Ok((idx, suboptimal)) => {
                if suboptimal {
                    warn!("swapchain suboptimal for surface (recreating swapchain)");
                    // TODO: recreate swapchain
                }
                self.frame_opened = true;
                self.current_image_idx = idx as usize;
            }
        }
        let cmd_buffer = self.cmd_buffers[self.current_frame_idx];
        unsafe {
            self.kew_device
                .begin_command_buffer(cmd_buffer, &vk::CommandBufferBeginInfo::default())
                .unwrap()
        }
        return Ok(cmd_buffer);
    }

    unsafe fn close_frame(&mut self, cmd_buffer: vk::CommandBuffer) {
        self.kew_device.end_command_buffer(cmd_buffer).unwrap();
        self.swapchain.submit_and_present(
            cmd_buffer,
            self.current_image_idx,
            self.current_frame_idx,
            &self.cmd_pool.queue,
        );
        self.frame_opened = false;
        self.current_frame_idx = (self.current_frame_idx + 1) % MAX_IN_FLIGHT_FRAMES;
    }
}

pub struct DockScene<'a> {
    vrt_buffer: &'a KewBuffer<'a>,
    idx_buffer: &'a KewBuffer<'a>,
    model_infos: Vec<KewModel>,
    pipeline: KewGfxPipeline<'a>,
    //vert_dset_layout: vk::DescriptorSetLayout,
    //frag_dset_layout: vk::DescriptorSetLayout,
}

impl<'a> DockScene<'a> {
    pub fn dummy(
        kew_device: &'a KewDevice,
        vrt_buffer: &'a KewBuffer<'a>,
        idx_buffer: &'a KewBuffer<'a>,
        vert_shader: &KewShader,
        frag_shader: &KewShader,
        render_pass: &vk::RenderPass,
    ) -> Self {
        let pipeline = KewGfxPipeline::new(
            kew_device,
            &PIPELINE_CONFIGS[FLAT_VERT_CONFIG],
            Self::create_pipeline_layout(kew_device),
            vert_shader,
            frag_shader,
            render_pass,
        );
        Self {
            vrt_buffer,
            idx_buffer,
            model_infos: Vec::new(),
            pipeline,
        }
    }

    pub unsafe fn record_cmd_buffer(&self, cmd_buffer: vk::CommandBuffer) {
        self.pipeline.bind_pipeline(cmd_buffer);
        for model in &self.model_infos {
            model.bind(self.pipeline.kew_device, cmd_buffer, &self.vrt_buffer, &self.idx_buffer);
            model.draw(self.pipeline.kew_device, cmd_buffer);
        }
    }

    pub fn add_model(&mut self, model: KewModel) {
        self.model_infos.push(model);
    }

    fn create_pipeline_layout(kew_device: &KewDevice) -> vk::PipelineLayout {
        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default();
        unsafe {
            kew_device
                .create_pipeline_layout(&pipeline_layout_info, None)
                .unwrap()
        }
    }
}

impl Drop for DockScene<'_> {
    fn drop(&mut self) {
        debug!("HELLO")
    }
}
