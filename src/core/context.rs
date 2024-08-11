use crate::core::ENABLE_VALIDATION_LAYERS;
use ash::ext::debug_utils;
#[allow(unused_imports)]
use ash::khr::{surface, wayland_surface, win32_surface};
use ash::vk::DebugUtilsMessageSeverityFlagsEXT as Severity;
use ash::vk::DebugUtilsMessageTypeFlagsEXT as Type;
use ash::{vk, Entry, Instance};
use log::{debug, error, info, warn};
use std::ffi::{c_void, CStr, CString};

pub struct KewContext {
    pub entry: Entry,
    pub instance: Instance,
    pub physical: vk::PhysicalDevice,
    pub mem_properties: vk::PhysicalDeviceMemoryProperties,
    #[cfg(debug_assertions)]
    debug_utils: Option<(debug_utils::Instance, vk::DebugUtilsMessengerEXT)>,
}

impl KewContext {
    pub fn new() -> Self {
        let entry: Entry = unsafe { Entry::load().expect("failed loading entry") };
        let instance = Self::create_instance(&entry);
        unsafe {
            #[cfg(debug_assertions)]
            let debug_utils = Self::create_debug_utils(&entry, &instance);
            let physical = Self::pick_physical_device(&instance);
            let mem_properties = instance.get_physical_device_memory_properties(physical);

            Self {
                entry,
                instance,
                physical,
                mem_properties,
                #[cfg(debug_assertions)]
                debug_utils,
            }
        }
    }

    fn create_instance(entry: &Entry) -> Instance {
        let kew_str = CString::new("kew").unwrap();
        let version = get_version();
        let app_info = vk::ApplicationInfo::default()
            .api_version(vk::API_VERSION_1_3)
            .application_name(&kew_str)
            .application_version(version)
            .engine_name(&kew_str)
            .engine_version(version);
        let extensions = Self::get_extensions();

        let mut create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&extensions);
        unsafe {
            let instance = if ENABLE_VALIDATION_LAYERS {
                let layer_names =
                    vec![
                        CStr::from_bytes_with_nul_unchecked(b"VK_LAYER_KHRONOS_validation\0")
                            .as_ptr(),
                    ];
                info!("loaded {} validation layer(s)", layer_names.len());
                create_info = create_info.enabled_layer_names(&layer_names);
                entry.create_instance(&create_info, None)
            } else {
                entry.create_instance(&create_info, None)
            };
            instance.expect("failed creating instance")
        }
    }

    fn get_extensions() -> Vec<*const i8> {
        let mut extensions: Vec<*const i8> = Vec::new();
        extensions.push(surface::NAME.as_ptr());
        #[cfg(target_os = "windows")]
        extensions.push(win32_surface::NAME.as_ptr());
        #[cfg(target_os = "linux")]
        extensions.push(wayland_surface::NAME.as_ptr());
        if cfg!(debug_assertions) {
            extensions.push(debug_utils::NAME.as_ptr());
        }
        info!("loaded {} instance extension(s)", extensions.len());
        extensions
    }

    unsafe fn pick_physical_device(instance: &Instance) -> vk::PhysicalDevice {
        let devices = instance
            .enumerate_physical_devices()
            .expect("failed to enumerate physical devices");
        info!("instance enumerated {} device(s)", devices.len());

        match devices.iter().find(|device| {
            let device_type = instance
                .get_physical_device_properties(**device)
                .device_type;
            device_type == vk::PhysicalDeviceType::DISCRETE_GPU
        }) {
            Some(selected) => *selected,
            None => {
                warn!("no discrete gpu found, falling back to first enumerated");
                *devices.first().expect("no devices found")
            }
        }
    }

    #[cfg(debug_assertions)]
    unsafe fn create_debug_utils(
        entry: &Entry,
        instance: &Instance,
    ) -> Option<(debug_utils::Instance, vk::DebugUtilsMessengerEXT)> {
        let create_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(Severity::ERROR | Severity::WARNING)
            .message_type(Type::GENERAL | Type::VALIDATION | Type::PERFORMANCE)
            .pfn_user_callback(Some(vulkan_debug_callback));
        let loader = debug_utils::Instance::new(entry, instance);
        let messenger = match loader.create_debug_utils_messenger(&create_info, None) {
            Ok(callback) => callback,
            Err(_) => {
                error!("failed creating debug utils messenger");
                return None;
            }
        };
        Some((loader, messenger))
    }
}

impl Drop for KewContext {
    fn drop(&mut self) {
        debug!("dropping KewContext");
        unsafe {
            #[cfg(debug_assertions)]
            if let Some((utils, messenger)) = self.debug_utils.take() {
                utils.destroy_debug_utils_messenger(messenger, None);
            }
            self.instance.destroy_instance(None);
        }
    }
}

fn get_version() -> u32 {
    let mut version_parts = env!("CARGO_PKG_VERSION").split('.');
    let major = version_parts
        .next()
        .expect("Invalid version format")
        .parse::<u32>()
        .expect("Invalid major version");
    let minor = version_parts
        .next()
        .expect("Invalid version format")
        .parse::<u32>()
        .expect("Invalid minor version");
    let patch = version_parts
        .next()
        .expect("Invalid version format")
        .parse::<u32>()
        .expect("Invalid patch version");
    vk::make_api_version(0, major, minor, patch)
}

#[cfg(debug_assertions)]
pub unsafe extern "system" fn vulkan_debug_callback(
    s_flags: Severity,
    t_flags: Type,
    callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut c_void,
) -> vk::Bool32 {
    let msg = CStr::from_ptr((*callback_data).p_message);
    match s_flags {
        Severity::ERROR => error!("{:?} - {:?}", t_flags, msg),
        Severity::INFO => info!("{:?} - {:?}", t_flags, msg),
        Severity::WARNING => warn!("{:?} - {:?}", t_flags, msg),
        _ => {}
    }
    vk::FALSE // TRUE == skip call to driver
}
