use crate::core::context::KewContext;
use crate::core::device::KewDevice;
use crate::core::{PREFERRED_SURFACE_COLORS, PREFERRED_SURFACE_FORMAT};
use ash::khr::{surface, swapchain};
use ash::vk;
use log;
use log::{debug, warn};

pub const MAX_IN_FLIGHT_FRAMES: usize = 2;

type FrameAttachment = (vk::Image, vk::ImageView);
struct KewFrameBundle<'a> {
    kew_device: &'a KewDevice,
    framebuffer: vk::Framebuffer,
    swapchain_attachment: FrameAttachment,
}

impl Drop for KewFrameBundle<'_> {
    fn drop(&mut self) {
        unsafe {
            self.kew_device
                .destroy_image_view(self.swapchain_attachment.1, None);
            self.kew_device.destroy_framebuffer(self.framebuffer, None);
        }
    }
}

pub struct KewSwapchain<'a> {
    kew_device: &'a KewDevice,
    present_queue: vk::Queue,
    swapchain_loader: swapchain::Device,
    swapchain: vk::SwapchainKHR,
    swapchain_extent: vk::Extent2D,
    frame_bundles: Vec<KewFrameBundle<'a>>,
    image_available_semaphores: [vk::Semaphore; MAX_IN_FLIGHT_FRAMES],
    render_finished_semaphores: [vk::Semaphore; MAX_IN_FLIGHT_FRAMES],
    frame_in_flight_fences: [vk::Fence; MAX_IN_FLIGHT_FRAMES],
    pub image_format: vk::Format,
    pub render_pass: vk::RenderPass,
}

impl<'a> KewSwapchain<'a> {
    pub fn new(
        kew_device: &'a KewDevice,
        surface_loader: &surface::Instance,
        surface: vk::SurfaceKHR,
        window_extent: vk::Extent2D,
        prs_queue_idx: u32,
    ) -> Self {
        unsafe {
            let surface_format =
                Self::pick_surface_format(&kew_device.context, surface_loader, surface);
            let render_pass = Self::create_render_pass(&kew_device, surface_format.format);
            let capabilities = surface_loader
                .get_physical_device_surface_capabilities(kew_device.context.physical, surface)
                .unwrap();
            let swapchain_extent = match capabilities.current_extent.width {
                u32::MAX => window_extent,
                _ => capabilities.current_extent,
            };
            let present_mode = surface_loader
                .get_physical_device_surface_present_modes(kew_device.context.physical, surface)
                .unwrap()
                .iter()
                .map(|pm| *pm)
                .find(|present_mode| *present_mode == vk::PresentModeKHR::IMMEDIATE)
                .unwrap_or_else(|| {
                    warn!("desired present mode unavailable (default FIFO)");
                    vk::PresentModeKHR::FIFO
                });
            let present_queue = kew_device.get_device_queue(prs_queue_idx, 0);


            let create_info = vk::SwapchainCreateInfoKHR::default()
                .surface(surface)
                .min_image_count(capabilities.min_image_count + 1)
                .image_color_space(surface_format.color_space)
                .image_format(surface_format.format)
                .image_extent(swapchain_extent)
                .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                .pre_transform(capabilities.current_transform)
                .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                .present_mode(present_mode)
                .clipped(true)
                .image_array_layers(1);
            let swapchain_loader = swapchain::Device::new(&kew_device.context.instance, kew_device);
            let swapchain = swapchain_loader
                .create_swapchain(&create_info, None)
                .expect("failed to create swapchain");

            let frame_bundles = Self::create_frame_bundles(
                &kew_device,
                swapchain_extent,
                &swapchain_loader,
                swapchain,
                surface_format.format,
                render_pass,
            );

            let mut image_available_semaphores: [vk::Semaphore; MAX_IN_FLIGHT_FRAMES] =
                [vk::Semaphore::null(); MAX_IN_FLIGHT_FRAMES];
            let mut render_finished_semaphores: [vk::Semaphore; MAX_IN_FLIGHT_FRAMES] =
                [vk::Semaphore::null(); MAX_IN_FLIGHT_FRAMES];
            let semaphore_info = vk::SemaphoreCreateInfo::default();
            let mut frame_in_flight_fences: [vk::Fence; MAX_IN_FLIGHT_FRAMES] =
                [vk::Fence::null(); MAX_IN_FLIGHT_FRAMES];
            let fence_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
            for i in 0..MAX_IN_FLIGHT_FRAMES {
                image_available_semaphores[i] = kew_device
                    .create_semaphore(&semaphore_info, None)
                    .expect("failed to create image available semaphore");
                render_finished_semaphores[i] = kew_device
                    .create_semaphore(&semaphore_info, None)
                    .expect("failed to create render finished semaphore");
                frame_in_flight_fences[i] = kew_device
                    .create_fence(&fence_info, None)
                    .expect("failed to create frame in flight fences");
            }

            Self {
                kew_device,
                present_queue,
                swapchain_loader,
                swapchain,
                swapchain_extent,
                frame_bundles,
                image_available_semaphores,
                render_finished_semaphores,
                frame_in_flight_fences,
                image_format: surface_format.format,
                render_pass,
            }
        }
    }

    pub unsafe fn begin_render_pass(&self, cmd_buffer: vk::CommandBuffer, image_idx: usize) {
        let clear_vals = [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.01, 0.01, 0.01, 0.01],
            },
        }];
        let begin_info = vk::RenderPassBeginInfo::default()
            .render_pass(self.render_pass)
            .framebuffer(self.frame_bundles[image_idx].framebuffer)
            .render_area(vk::Rect2D {
                offset: vk::Offset2D::default(),
                extent: self.swapchain_extent,
            })
            .clear_values(&clear_vals);
        self.kew_device
            .cmd_begin_render_pass(cmd_buffer, &begin_info, vk::SubpassContents::INLINE);

        let viewport = vk::Viewport::default()
            .width(self.swapchain_extent.width as f32)
            .height(self.swapchain_extent.height as f32);
        let scissor = vk::Rect2D::default().extent(self.swapchain_extent);
        self.kew_device.cmd_set_viewport(cmd_buffer, 0, &[viewport]);
        self.kew_device.cmd_set_scissor(cmd_buffer, 0, &[scissor]);
    }

    pub unsafe fn end_render_pass(&self, cmd_buffer: vk::CommandBuffer) {
        self.kew_device.cmd_end_render_pass(cmd_buffer);
    }

    pub unsafe fn submit_and_present(
        &self,
        cmd_buffer: vk::CommandBuffer,
        image_idx: usize,
        frame_idx: usize,
        gfx_queue: &vk::Queue,
    ) {
        let wait_semaphores = [self.image_available_semaphores[frame_idx]];
        let ping_semaphores = [self.render_finished_semaphores[frame_idx]];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let cmd_buffers = [cmd_buffer];
        let submit_info = vk::SubmitInfo::default()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(&cmd_buffers)
            .signal_semaphores(&ping_semaphores);

        self.kew_device
            .reset_fences(&[self.frame_in_flight_fences[frame_idx]])
            .unwrap();
        self.kew_device
            .queue_submit(
                *gfx_queue,
                &[submit_info],
                self.frame_in_flight_fences[frame_idx],
            )
            .unwrap();

        let swapchains = [self.swapchain];
        let image_idxs = [image_idx as u32];
        let wait_semaphores = [self.render_finished_semaphores[frame_idx]];
        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&wait_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_idxs);
        self.swapchain_loader
            .queue_present(self.present_queue, &present_info)
            .expect("failed to present swapchain image");
    }

    pub unsafe fn frame_in_use(&self, frame_idx: usize) -> bool {
        !self
            .kew_device
            .get_fence_status(self.frame_in_flight_fences[frame_idx])
            .unwrap()
    }

    unsafe fn create_frame_bundles(
        kew_device: &'a KewDevice,
        swapchain_extent: vk::Extent2D,
        swapchain_loader: &swapchain::Device,
        swapchain: vk::SwapchainKHR,
        image_format: vk::Format,
        render_pass: vk::RenderPass,
    ) -> Vec<KewFrameBundle<'a>> {
        let swapchain_images = swapchain_loader.get_swapchain_images(swapchain).unwrap();
        let swapchain_views = swapchain_images
            .iter()
            .map(|image| {
                let create_info = vk::ImageViewCreateInfo::default()
                    .image(*image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(image_format)
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    });
                kew_device.create_image_view(&create_info, None).unwrap()
            })
            .collect::<Vec<vk::ImageView>>();

        let mut framebundles: Vec<KewFrameBundle> = Vec::with_capacity(swapchain_images.len());
        for i in 0..swapchain_images.len() {
            let attachments = [swapchain_views[i]];
            let create_info = vk::FramebufferCreateInfo::default()
                .render_pass(render_pass)
                .attachments(&attachments)
                .width(swapchain_extent.width)
                .height(swapchain_extent.height)
                .layers(1);
            let framebuffer = kew_device.create_framebuffer(&create_info, None).unwrap();
            framebundles.push(KewFrameBundle {
                kew_device,
                framebuffer,
                swapchain_attachment: (swapchain_images[i], swapchain_views[i]),
            });
        }
        framebundles
    }

    unsafe fn create_render_pass(
        kew_device: &KewDevice,
        swapchain_image_format: vk::Format,
    ) -> vk::RenderPass {
        let color_attachment = vk::AttachmentDescription::default()
            .format(swapchain_image_format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);
        let attachments = [color_attachment];

        let color_attachment_ref = vk::AttachmentReference::default()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
        let attachment_refs = [color_attachment_ref];

        let subpass = vk::SubpassDescription::default()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&attachment_refs);
        let subpasses = [subpass];

        let create_info = vk::RenderPassCreateInfo::default()
            .attachments(&attachments)
            .subpasses(&subpasses);
        kew_device.create_render_pass(&create_info, None).unwrap()
    }

    unsafe fn pick_surface_format(
        context: &KewContext,
        surface_loader: &surface::Instance,
        surface: vk::SurfaceKHR,
    ) -> vk::SurfaceFormatKHR {
        let formats = surface_loader
            .get_physical_device_surface_formats(context.physical, surface)
            .unwrap();
        let format = formats
            .iter()
            .find(|surface_format| {
                surface_format.format == PREFERRED_SURFACE_FORMAT
                    && surface_format.color_space == PREFERRED_SURFACE_COLORS
            })
            .unwrap_or_else(|| {
                warn!("did not find desired surface format (defaulting to first enumerated)");
                formats.first().unwrap()
            });
        debug!("surface format: {:?}", format);
        *format
    }

    pub unsafe fn next_image_idx(&self, frame_idx: usize) -> Result<(u32, bool), vk::Result> {
        self.swapchain_loader.acquire_next_image(
            self.swapchain,
            u64::MAX,
            self.image_available_semaphores[frame_idx],
            vk::Fence::null(),
        )
    }
}

impl Drop for KewSwapchain<'_> {
    fn drop(&mut self) {
        debug!("dropping KewSwapchain");
        unsafe {
            self.kew_device
                .wait_for_fences(&self.frame_in_flight_fences, true, u64::MAX)
                .unwrap();
            self.frame_in_flight_fences
                .iter()
                .for_each(|fence| self.kew_device.destroy_fence(*fence, None));

            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
            self.kew_device.destroy_render_pass(self.render_pass, None);

            self.image_available_semaphores
                .iter()
                .for_each(|s| self.kew_device.destroy_semaphore(*s, None));
            self.render_finished_semaphores
                .iter()
                .for_each(|s| self.kew_device.destroy_semaphore(*s, None));
        }
    }
}
