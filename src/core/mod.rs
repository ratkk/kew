use ash::vk;

pub mod buffer;
pub mod command;
pub mod context;
pub mod descriptor;
pub mod device;
pub mod image;
pub mod memory;
pub mod model;
pub mod pipeline;
pub mod shader;
pub mod surface;
pub mod swapchain;

const ENABLE_VALIDATION_LAYERS: bool = cfg!(debug_assertions);
const PREFERRED_SURFACE_FORMAT: vk::Format = vk::Format::R8G8B8A8_UNORM;
const PREFERRED_SURFACE_COLORS: vk::ColorSpaceKHR = vk::ColorSpaceKHR::SRGB_NONLINEAR;
