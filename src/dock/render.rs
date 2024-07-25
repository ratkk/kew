use std::ffi::CStr;
use std::sync::{mpsc};
use std::{sync::mpsc::Sender, thread};

use ash::khr::surface;
use ash::vk;
use log::{debug, error, warn};
use winit::raw_window_handle::{RawDisplayHandle, RawWindowHandle};

use crate::core::command::KewCommandPool;
use crate::core::context::KewContext;
use crate::core::device::{KewDevice, KewQueueIndices};
use crate::core::pipeline::{GfxPipelineConfig, KewGfxPipeline};
use crate::core::shader::{KewShader, ShaderStageConfig};
use crate::core::surface as kew_surface;
use crate::core::swapchain::{KewSwapchain, MAX_IN_FLIGHT_FRAMES};

const VERT_SHADER_CONFIG: ShaderStageConfig<0> = unsafe {
    ShaderStageConfig {
        entry_name: CStr::from_bytes_with_nul_unchecked(b"main\0"),
        path: "./shader/kew.vert.spv",
        bindings: [],
        stage: vk::ShaderStageFlags::VERTEX,
        create_flags: vk::PipelineShaderStageCreateFlags::empty(),
    }
};
const FRAG_SHADER_CONFIG: ShaderStageConfig<0> = unsafe {
    ShaderStageConfig {
        entry_name: CStr::from_bytes_with_nul_unchecked(b"main\0"),
        path: "./shader/kew.frag.spv",
        bindings: [],
        stage: vk::ShaderStageFlags::FRAGMENT,
        create_flags: vk::PipelineShaderStageCreateFlags::empty(),
    }
};

pub enum RenderUpdate {
    REDRAW,
}

pub fn start_render_thread(
    raw_display_handle: RawDisplayHandle,
    raw_window_handle: RawWindowHandle,
    window_extent: vk::Extent2D,
) -> Sender<RenderUpdate> {
    let (tx, rx) = mpsc::channel();

    let kew_context = KewContext::new();
    let (surface_loader, surface) = unsafe {
        kew_surface::create_surface(
            &kew_context.entry,
            &kew_context.instance,
            raw_display_handle,
            raw_window_handle,
        )
    };

    thread::spawn(move || {
        let mut queue_indices = KewQueueIndices::new(&kew_context);
        queue_indices.add_present_queue(&kew_context, &surface_loader, surface);
        let kew_device = KewDevice::new(kew_context, &queue_indices);

        let vert_shader = KewShader::new(&kew_device, &VERT_SHADER_CONFIG);
        let frag_shader = KewShader::new(&kew_device, &FRAG_SHADER_CONFIG);
        let mut renderer = DockRenderer::new(
            &kew_device,
            &surface_loader,
            surface,
            window_extent,
            queue_indices,
            &vert_shader,
            &frag_shader,
        );

        loop {
            match rx.recv() {
                Ok(update) => handle_render_update(&mut renderer, update),
                Err(_) => {
                    error!("render thread error mpsc message received (dropping thread)");
                    unsafe {
                        drop(renderer);
                        surface_loader.destroy_surface(surface, None);
                    }
                    return;
                }
            }
        }
    });
    tx
}

fn handle_render_update(renderer: &mut DockRenderer, update: RenderUpdate) {
    match update {
        RenderUpdate::REDRAW => {
            if let Some(cmd_buffer) = renderer.open_frame() {
                renderer.begin_render_pass(cmd_buffer);
                unsafe {
                    renderer.gfx_pipeline.bind(cmd_buffer);
                    renderer.draw(cmd_buffer);
                }
                renderer.end_render_pass(cmd_buffer);
                renderer.close_frame();
            }
        }
    }
}

struct DockRenderer<'a> {
    kew_device: &'a KewDevice,
    swapchain: KewSwapchain<'a>,
    present_queue: vk::Queue,
    gfx_cmd_pool: KewCommandPool<'a>,
    gfx_cmd_buffers: Vec<vk::CommandBuffer>,
    gfx_pipeline: KewGfxPipeline<'a>,
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
        queue_indices: KewQueueIndices,
        vert_shader: &KewShader,
        frag_shader: &KewShader,
    ) -> Self {
        let present_queue =
            unsafe { kew_device.get_device_queue(queue_indices.prs_idx.unwrap(), 0) };
        let gfx_cmd_pool = KewCommandPool::new(&kew_device, queue_indices.gfx_idx);
        let gfx_cmd_buffers = gfx_cmd_pool
            .create_command_buffers(vk::CommandBufferLevel::PRIMARY, MAX_IN_FLIGHT_FRAMES as u32);

        let swapchain = KewSwapchain::new(&kew_device, &surface_loader, surface, window_extent);
        let gfx_pipeline = KewGfxPipeline::new(
            &kew_device,
            GfxPipelineConfig::default(),
            create_pipeline_layout(&kew_device),
            &vert_shader,
            &frag_shader,
            &swapchain.render_pass,
        );

        Self {
            kew_device,
            swapchain,
            present_queue,
            gfx_cmd_pool,
            gfx_cmd_buffers,
            gfx_pipeline,
            current_frame_idx: 0,
            current_image_idx: 0,
            frame_opened: false,
        }
    }

    pub fn open_frame(&mut self) -> Option<vk::CommandBuffer> {
        assert!(
            !self.frame_opened,
            "cannot open frame: there is already an open frame"
        );
        unsafe {
            if self.swapchain.is_frame_available(self.current_frame_idx) {
                debug!("frame dropped");
                return None;
            }
        }

        let image_idx_result = unsafe { self.swapchain.next_image_idx(self.current_frame_idx) };
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

        let cmd_buffer = self.gfx_cmd_buffers[self.current_frame_idx];
        unsafe {
            self.kew_device
                .begin_command_buffer(cmd_buffer, &vk::CommandBufferBeginInfo::default())
                .unwrap()
        }
        return Some(cmd_buffer);
    }

    pub fn close_frame(&mut self) {
        assert!(self.frame_opened);
        let cmd_buffer = self.current_cmd_buffer();
        unsafe {
            self.kew_device.end_command_buffer(cmd_buffer).unwrap();
            self.swapchain.submit_and_present(
                cmd_buffer,
                self.current_image_idx,
                self.current_frame_idx,
                &self.gfx_cmd_pool.queue,
                &self.present_queue,
            );
        }
        self.frame_opened = false;
        self.current_frame_idx = (self.current_frame_idx + 1) % MAX_IN_FLIGHT_FRAMES;
    }

    pub fn begin_render_pass(&self, cmd_buffer: vk::CommandBuffer) {
        assert_eq!(cmd_buffer, self.current_cmd_buffer());
        unsafe {
            self.swapchain
                .begin_render_pass(cmd_buffer, self.current_image_idx);
        }
    }

    pub fn end_render_pass(&self, cmd_buffer: vk::CommandBuffer) {
        assert_eq!(cmd_buffer, self.current_cmd_buffer());
        unsafe {
            self.swapchain.end_render_pass(cmd_buffer);
        }
    }

    fn current_cmd_buffer(&self) -> vk::CommandBuffer {
        assert!(
            self.frame_opened,
            "getting command buffer requires started frame"
        );
        self.gfx_cmd_buffers[self.current_frame_idx]
    }

    unsafe fn draw(&self, cmd_buffer: vk::CommandBuffer) {
        self.kew_device.cmd_draw(cmd_buffer, 3, 1, 0, 0);
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
