use crate::core::device::KewDevice;
use ash::vk;
use log::debug;

pub struct KewDescriptorPool<'a> {
    kew_device: &'a KewDevice,
    pool: vk::DescriptorPool,
}

impl<'a> KewDescriptorPool<'a> {
    pub fn new(
        kew_device: &'a KewDevice,
        pool_sizes: &Vec<vk::DescriptorPoolSize>,
        pool_flags: vk::DescriptorPoolCreateFlags,
        max_sets: u32,
    ) -> Self {
        let create_info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(pool_sizes.as_slice())
            .max_sets(max_sets)
            .flags(pool_flags);
        let pool = unsafe {
            kew_device
                .create_descriptor_pool(&create_info, None)
                .expect("failed to create descriptor pool")
        };
        Self { kew_device, pool }
    }

    pub unsafe fn allocate_descriptor_set(
        &self,
        descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> vk::DescriptorSet {
        let binding = [descriptor_set_layout];
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.pool)
            .set_layouts(&binding);
        self.kew_device
            .allocate_descriptor_sets(&alloc_info)
            .expect("failed to allocate descriptor set")[0]
    }
}

impl Drop for KewDescriptorPool<'_> {
    fn drop(&mut self) {
        debug!("dropping KewDescriptorPool");
        unsafe {
            self.kew_device.destroy_descriptor_pool(self.pool, None);
        }
    }
}

pub struct KewDescriptorPoolBuilder {
    pool_sizes: Vec<vk::DescriptorPoolSize>,
    pool_flags: vk::DescriptorPoolCreateFlags,
    max_sets: u32,
}

impl<'a> KewDescriptorPoolBuilder {
    pub fn new(max_sets: u32) -> Self {
        Self {
            pool_sizes: Vec::<vk::DescriptorPoolSize>::new(),
            pool_flags: vk::DescriptorPoolCreateFlags::empty(),
            max_sets,
        }
    }

    pub fn add_pool_size(
        mut self,
        descriptor_type: vk::DescriptorType,
        count: u32,
    ) -> KewDescriptorPoolBuilder {
        self.pool_sizes.push(vk::DescriptorPoolSize {
            ty: descriptor_type,
            descriptor_count: count,
        });
        self
    }

    pub fn build(self, kew_device: &'a KewDevice) -> KewDescriptorPool<'a> {
        KewDescriptorPool::new(kew_device, &self.pool_sizes, self.pool_flags, self.max_sets)
    }
}
