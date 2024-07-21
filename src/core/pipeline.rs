use crate::core::command::KewCommandPool;
use crate::core::device::KewDevice;
use crate::core::shader::KewShader;
use ash::vk;
use log::debug;

pub struct KewCmpPipeline<'a> {
    kew_device: &'a KewDevice,
    layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
}

impl<'a> KewCmpPipeline<'a> {
    pub fn new(kew_device: &'a KewDevice, shader: &KewShader) -> Self {
        let layout = unsafe {
            let descriptor_set_layouts = &[shader.descriptor_set_layout];
            let create_info =
                vk::PipelineLayoutCreateInfo::default().set_layouts(descriptor_set_layouts);
            kew_device
                .create_pipeline_layout(&create_info, None)
                .expect("failed to create pipeline layout")
        };
        let pipeline = Self::create_pipeline(&kew_device, layout, shader.shader_stage_info);
        Self {
            kew_device,
            layout,
            pipeline,
        }
    }

    fn create_pipeline(
        kew_device: &KewDevice,
        layout: vk::PipelineLayout,
        comp_shader_stage: vk::PipelineShaderStageCreateInfo,
    ) -> vk::Pipeline {
        let create_info = vk::ComputePipelineCreateInfo::default()
            .flags(vk::PipelineCreateFlags::empty())
            .stage(comp_shader_stage)
            .layout(layout);
        unsafe {
            kew_device
                .create_compute_pipelines(
                    vk::PipelineCache::null(),
                    std::slice::from_ref(&create_info),
                    None,
                )
                .expect("failed to create compute pipeline")[0]
        }
    }

    pub fn get_bound_cmd_buffer(
        &self,
        command_pool: &KewCommandPool,
        descriptor_set: vk::DescriptorSet,
    ) -> vk::CommandBuffer {
        let command_buffer = command_pool.get_command_buffer();
        unsafe {
            self.kew_device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                self.pipeline,
            );
            self.kew_device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                self.layout,
                0,
                &[descriptor_set],
                &[],
            )
        }
        command_buffer
    }
}

impl Drop for KewCmpPipeline<'_> {
    fn drop(&mut self) {
        debug!("dropping KewCmpPipeline");
        unsafe {
            self.kew_device.destroy_pipeline_layout(self.layout, None);
            self.kew_device.destroy_pipeline(self.pipeline, None);
        }
    }
}

pub struct BlendInfo {
    src_factor: vk::BlendFactor,
    dst_factor: vk::BlendFactor,
    operation: vk::BlendOp,
}

pub struct ColorTarget {
    color_blend: Option<BlendInfo>,
    alpha_blend: Option<BlendInfo>,
    write_mask: vk::ColorComponentFlags,
}

pub struct PrimitiveState {
    topology: vk::PrimitiveTopology,
    restart: bool,
    polygon_mode: vk::PolygonMode,
    depth_clamp: bool,
    cull_mode: vk::CullModeFlags,
    front_face: vk::FrontFace,
}

pub struct GfxPipelineConfig<const N: usize> {
    primitive: PrimitiveState,
    color_targets: [ColorTarget; N],
    // multisample
    // depth stencil
}

impl Default for GfxPipelineConfig<1> {
    fn default() -> Self {
        GfxPipelineConfig {
            primitive: PrimitiveState {
                topology: vk::PrimitiveTopology::TRIANGLE_LIST,
                restart: false,
                polygon_mode: vk::PolygonMode::FILL,
                depth_clamp: false,
                cull_mode: vk::CullModeFlags::NONE,
                front_face: vk::FrontFace::CLOCKWISE,
            },
            color_targets: [ColorTarget {
                color_blend: None,
                alpha_blend: None,
                write_mask: vk::ColorComponentFlags::RGBA,
            }],
        }
    }
}

pub struct KewGfxPipeline<'a> {
    kew_device: &'a KewDevice,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
}

impl<'a> KewGfxPipeline<'a> {
    pub fn new<const N: usize>(
        kew_device: &'a KewDevice,
        config: GfxPipelineConfig<N>,
        layout: vk::PipelineLayout,
        vert_shader: &KewShader,
        frag_shader: &KewShader,
        render_pass: &vk::RenderPass,
    ) -> Self {
        let pstages = [vert_shader.shader_stage_info, frag_shader.shader_stage_info];
        let dstates = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];

        let vin = vk::PipelineVertexInputStateCreateInfo::default();
        let ina = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(config.primitive.topology)
            .primitive_restart_enable(config.primitive.restart);
        let vps = vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);
        let ras = vk::PipelineRasterizationStateCreateInfo::default()
            .polygon_mode(config.primitive.polygon_mode)
            .line_width(1.0)
            .depth_clamp_enable(config.primitive.depth_clamp)
            .cull_mode(config.primitive.cull_mode)
            .front_face(config.primitive.front_face)
            .depth_bias_enable(false);
        let mus = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1)
            .sample_shading_enable(false);

        let mut blend_attachments = Vec::with_capacity(config.color_targets.len());
        for target in config.color_targets {
            let mut attachment = vk::PipelineColorBlendAttachmentState::default()
                .color_write_mask(target.write_mask);
            if target.color_blend.is_some() || target.alpha_blend.is_some() {
                attachment = attachment.blend_enable(true);
                if let Some(blend_info) = target.color_blend {
                    attachment = attachment
                        .src_color_blend_factor(blend_info.src_factor)
                        .dst_color_blend_factor(blend_info.dst_factor)
                        .color_blend_op(blend_info.operation);
                }
                if let Some(blend_info) = target.alpha_blend {
                    attachment = attachment
                        .src_alpha_blend_factor(blend_info.src_factor)
                        .dst_alpha_blend_factor(blend_info.dst_factor)
                        .alpha_blend_op(blend_info.operation);
                }
            }
            blend_attachments.push(attachment);
        }
        debug!("loaded {} blend attachment(s)", blend_attachments.len());
        let cbl =
            vk::PipelineColorBlendStateCreateInfo::default().attachments(&blend_attachments);
        let dys =
            vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dstates);

        let create_info = vk::GraphicsPipelineCreateInfo::default()
            .layout(layout)
            .stages(&pstages)
            .vertex_input_state(&vin)
            .input_assembly_state(&ina)
            .viewport_state(&vps)
            .rasterization_state(&ras)
            .multisample_state(&mus)
            .color_blend_state(&cbl)
            .dynamic_state(&dys)
            .render_pass(*render_pass);

        let pipeline = unsafe {
            kew_device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[create_info], None)
                .unwrap()[0]
        };
        Self {
            kew_device,
            pipeline,
            pipeline_layout: layout,
        }
    }

    pub unsafe fn bind(&self, cmd_buffer: vk::CommandBuffer) {
        self.kew_device.cmd_bind_pipeline(
            cmd_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.pipeline,
        );
    }
}

impl Drop for KewGfxPipeline<'_> {
    fn drop(&mut self) {
        debug!("dropping KewGfxPipeline");
        unsafe {
            self.kew_device.destroy_pipeline(self.pipeline, None);
            self.kew_device.destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}
