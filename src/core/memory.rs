use crate::core::device::KewDevice;
use ash::vk;
use log::{debug, warn};
use std::cell::Cell;
use std::ffi::c_void;
use std::ptr;

pub struct KewMemoryBinding<'a> {
    pub memory: &'a KewMemory<'a>,
    pub offset: vk::DeviceSize,
}

pub struct KewMemory<'a> {
    pub kew_device: &'a KewDevice,
    pub memory: vk::DeviceMemory,
    pub b_size: vk::DeviceSize,
    mapped: Cell<*mut c_void>,
}

impl<'a> KewMemory<'a> {
    pub fn new(
        kew_device: &'a KewDevice,
        b_size: u64,
        memory_reqs: vk::MemoryRequirements,
        memory_flag: vk::MemoryPropertyFlags,
    ) -> Self {
        let info = vk::MemoryAllocateInfo::default()
            .allocation_size(b_size)
            .memory_type_index(
                kew_device
                    .find_memory_type(&memory_reqs, memory_flag)
                    .unwrap(),
            );
        let memory = unsafe { kew_device.allocate_memory(&info, None).unwrap() };
        Self {
            kew_device,
            memory,
            mapped: Cell::new(ptr::null_mut()),
            b_size,
        }
    }

    pub unsafe fn map(&self, b_size: vk::DeviceSize, offset: vk::DeviceSize) {
        if self.mapped.get().is_null() {
            let mapped = self
                .kew_device
                .map_memory(self.memory, offset, b_size, vk::MemoryMapFlags::empty())
                .expect("failed to map memory");
            self.mapped.set(mapped);
        } else {
            warn!("map call on mapped memory (skipped call)")
        }
    }

    #[allow(dead_code)]
    pub fn unmap(&self) {
        if !self.mapped.get().is_null() {
            unsafe {
                self.kew_device.unmap_memory(self.memory);
            }
            self.mapped.set(ptr::null_mut());
        } else {
            warn!("unmap call on unmapped memory (skipped call)")
        }
    }

    pub unsafe fn rd_visible_mem<T: Copy>(
        &self,
        data: &mut [T],
        b_size: vk::DeviceSize,
        offset: vk::DeviceSize,
    ) {
        let mapped = self.mapped.get();
        assert!(!mapped.is_null());
        assert!(
            b_size + offset <= self.b_size,
            "rd_visible_mem out of bounds: mem_size: {}, data_size: {}, data_offset: {}",
            self.b_size,
            b_size,
            offset
        );
        ptr::copy_nonoverlapping(
            (mapped as *const u8).add(offset as usize),
            data.as_ptr() as *mut u8,
            b_size as usize,
        );
    }

    pub unsafe fn wr_visible_mem<T: Copy>(
        &self,
        data: &[T],
        b_size: vk::DeviceSize,
        offset: vk::DeviceSize,
    ) {
        let mapped = self.mapped.get();
        assert!(
            !mapped.is_null(),
            "attempted wr_visible_mem on unmapped memory"
        );
        assert!(
            b_size + offset <= self.b_size,
            "wr_visible_mem out of bounds: mem_size: {}, data_size: {}, data_offset: {}",
            self.b_size,
            b_size,
            offset
        );
        ptr::copy_nonoverlapping(
            data.as_ptr() as *const u8,
            (mapped as *mut u8).add(offset as usize),
            b_size as usize,
        );
    }
}

impl Drop for KewMemory<'_> {
    fn drop(&mut self) {
        debug!("dropping KewMemory");
        unsafe {
            self.kew_device.free_memory(self.memory, None);
        }
    }
}
