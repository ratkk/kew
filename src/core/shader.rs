use crate::core::device::KewDevice;
use ash::vk;
use log::debug;
use std::{ffi::CStr, fs::File};

pub struct DescriptorSetLayoutBindingInfo {
    pub descriptor_type: vk::DescriptorType,
    pub descriptor_count: u32,
    pub stage_flags: vk::ShaderStageFlags,
}

pub struct ShaderStageConfig<const N: usize> {
    pub entry_name: &'static CStr,
    pub path: &'static str,
    pub bindings: [DescriptorSetLayoutBindingInfo; N],
    pub stage: vk::ShaderStageFlags,
    pub create_flags: vk::PipelineShaderStageCreateFlags,
}

impl<const N: usize> ShaderStageConfig<N> {
    pub fn build_dset_layout_bindings(&self) -> [vk::DescriptorSetLayoutBinding; N] {
        let mut bindings = [vk::DescriptorSetLayoutBinding::default(); N];
        for i in 0..N {
            bindings[i] = bindings[i].binding(i as u32);
            bindings[i] = bindings[i].descriptor_type(self.bindings[i].descriptor_type);
            bindings[i] = bindings[i].descriptor_count(self.bindings[i].descriptor_count);
            bindings[i] = bindings[i].stage_flags(self.bindings[i].stage_flags);
        }
        bindings
    }
}

pub struct KewShader<'a> {
    kew_device: &'a KewDevice,
    bindings: Vec<vk::DescriptorSetLayoutBinding<'a>>,
    pub shader_module: vk::ShaderModule,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub shader_stage_info: vk::PipelineShaderStageCreateInfo<'a>,
}

impl<'a> KewShader<'a> {
    pub fn new<const S: usize>(
        kew_device: &'a KewDevice,
        stage_config: &'a ShaderStageConfig<S>,
    ) -> Self {
        let bindings = stage_config.build_dset_layout_bindings();
        let create_info =
            vk::DescriptorSetLayoutCreateInfo::default().bindings(bindings.as_slice());
        let descriptor_set_layout = unsafe {
            kew_device
                .create_descriptor_set_layout(&create_info, None)
                .expect("failed to create descriptor set layout")
        };
        let shader_module = Self::create_shader_module(&kew_device, stage_config.path);

        let shader_stage_info = vk::PipelineShaderStageCreateInfo::default()
            .flags(stage_config.create_flags)
            .stage(stage_config.stage)
            .module(shader_module)
            .name(stage_config.entry_name);

        Self {
            kew_device,
            shader_module,
            descriptor_set_layout,
            shader_stage_info,
            bindings: bindings.to_vec(),
        }
    }

    fn create_shader_module(kew_device: &KewDevice, path: &'static str) -> vk::ShaderModule {
        let mut file = File::open(path).expect("failed to open shader file");
        let code = ash::util::read_spv(&mut file).expect("failed to read shader file");

        let create_info = vk::ShaderModuleCreateInfo::default().code(&code);
        unsafe {
            kew_device
                .create_shader_module(&create_info, None)
                .unwrap_or_else(|e| panic!("Failed to create shader module: {}", e))
        }
    }

    pub fn write_buffer(
        &self,
        binding: usize,
        buffer_info: vk::DescriptorBufferInfo,
        set: &vk::DescriptorSet,
    ) {
        let infos = vec![buffer_info];
        let writes = vec![vk::WriteDescriptorSet::default()
            .descriptor_type(self.bindings.get(binding).unwrap().descriptor_type)
            .dst_binding(binding as u32)
            .dst_set(*set)
            .buffer_info(&infos)];
        unsafe {
            self.kew_device
                .update_descriptor_sets(writes.as_slice(), &[]);
        }
    }

    pub fn write_image(
        &self,
        binding: usize,
        image_info: vk::DescriptorImageInfo,
        set: &vk::DescriptorSet,
    ) {
        let infos = vec![image_info];
        let writes = vec![vk::WriteDescriptorSet::default()
            .descriptor_type(self.bindings.get(binding).unwrap().descriptor_type)
            .dst_binding(binding as u32)
            .dst_set(*set)
            .image_info(&infos)];
        unsafe {
            self.kew_device
                .update_descriptor_sets(writes.as_slice(), &[])
        }
    }
}

impl Drop for KewShader<'_> {
    fn drop(&mut self) {
        debug!("dropping KewShader");
        unsafe {
            self.kew_device
                .destroy_shader_module(self.shader_module, None);
            self.kew_device
                .destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
    }
}
