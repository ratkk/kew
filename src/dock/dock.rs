use crate::core::buffer::KewBuffer;
use crate::core::command::KewCommandPool;
use crate::core::context::KewContext;
use crate::core::device::{KewDevice, KewQueueIndices};
use crate::core::memory::KewMemory;
use crate::core::model::KewModelVertexData;
use crate::core::pipeline::KewGfxPipeline;
use crate::core::shader::KewShader;
use crate::core::swapchain::{KewSwapchain, MAX_IN_FLIGHT_FRAMES};
use crate::dock::{DockErr, DockMessage};
use ash::khr::surface;
use ash::vk;
use crossbeam::channel::Receiver;
use log::{debug, error, warn};
use std::mem::{size_of, size_of_val, MaybeUninit};
use std::thread;
use ash::vk::SurfaceKHR;
use winit::raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use crate::dock::config::{FRAG_SHADER_CONFIG, NULL_VERT_CONFIG, PIPELINE_CONFIGS, VERT_SHADER_CONFIG};

const STAGE_MEM_IDX: usize = 0;
const MODEL_MEM_IDX: usize = 1;
const STAGE_MEM_SIZE: u64 = 1024;
const MODEL_MEM_SIZE: u64 = 1024;

pub fn init_dock(
    kew_device: &KewDevice,
    surface_loader: &surface::Instance,
    surface: SurfaceKHR,
    queue_indices: &KewQueueIndices,
    window_extent: vk::Extent2D,
    application_thread: Receiver<DockMessage>,
) {
    let mut memory_allocations: [MaybeUninit<KewMemory>; 2] =
        [const { MaybeUninit::uninit() }; 2];

    let mut renderer = DockRenderer::new(
        &kew_device,
        &surface_loader,
        surface,
        window_extent,
        queue_indices.prs_idx,
        queue_indices.gfx_idx,
    );

    let vert_shader = KewShader::new(&kew_device, &VERT_SHADER_CONFIG);
    let frag_shader = KewShader::new(&kew_device, &FRAG_SHADER_CONFIG);
    let dock_scene = DockScene::dummy(
        &kew_device,
        &vert_shader,
        &frag_shader,
        &renderer.swapchain.render_pass
    );

    memory_allocations[MODEL_MEM_IDX].write(KewMemory::new(&kew_device, MODEL_MEM_SIZE, 0));
    thread::scope(|scope| {

        // let mut stage_buffer = KewBuffer::new(
        //     &kew_device,
        //     STAGE_MEM_SIZE,
        //     vk::BufferUsageFlags::TRANSFER_SRC,
        // );
        // let stage_mem_type = kew_device
        //     .find_memory_type(
        //         &stage_buffer.get_memory_requirements(),
        //         vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_COHERENT
        //     )
        //     .unwrap();
        // memory_allocations[STAGE_MEM_IDX] = KewMemory::new(
        //     &kew_device,
        //     stage_buffer.get_memory_requirements().size,
        //     stage_mem_type
        // );
        // stage_buffer.bind_memory(&memory_allocations[STAGE_MEM_IDX], 0);

        //load_model(&kew_device);

        scope.spawn(|| loop {
            if let Ok(_) = application_thread.recv() {
                renderer.render_scene(&dock_scene);
                //load_model(&kew_device);
            } else {
                error!("dock render thread error mpsc message received (dropping thread)");
                unsafe {
                    surface_loader.destroy_surface(surface, None);
                    return;
                }
            }
        });
    });
}

// returns buffer object with loaded memory
fn load_model(kew_device: &KewDevice) {
    let model_data = KewModelVertexData::square();
    let mut object_buffer = KewBuffer::new(
        kew_device,
        size_of_val(&model_data) as u64,
        vk::BufferUsageFlags::STORAGE_BUFFER,
    );

    let object_mem_type = kew_device
        .find_memory_type(
            &object_buffer.get_memory_requirements(),
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )
        .unwrap();
    debug!("object memory type: {}", object_mem_type)
}

pub struct DockRenderer<'a> {
    kew_device: &'a KewDevice,
    swapchain: KewSwapchain<'a>,
    cmd_pool: KewCommandPool<'a>,
    cmd_buffers: [vk::CommandBuffer; MAX_IN_FLIGHT_FRAMES],
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

        Self {
            kew_device,
            swapchain,
            cmd_pool,
            cmd_buffers,
            current_frame_idx: 0,
            current_image_idx: 0,
            frame_opened: false,
        }
    }

    pub fn render_scene(&mut self, scene: &DockScene) {
        unsafe {
            if let Ok(cmd_buffer) = self.open_frame() {
                self.swapchain.begin_render_pass(cmd_buffer, self.current_image_idx);
                scene.pipeline.bind(cmd_buffer);
                self.kew_device.cmd_draw(cmd_buffer, 3, 1, 0, 0);
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
    pipeline: KewGfxPipeline<'a>,
}

impl<'a> DockScene<'a> {
    pub fn dummy(
        kew_device: &'a KewDevice,
        vert_shader: &KewShader,
        frag_shader: &KewShader,
        render_pass: &vk::RenderPass,
    ) -> Self {
        let pipeline = KewGfxPipeline::new(
            kew_device,
            &PIPELINE_CONFIGS[NULL_VERT_CONFIG],
            Self::create_pipeline_layout(kew_device),
            vert_shader,
            frag_shader,
            render_pass
        );
        Self {
            pipeline
        }
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
