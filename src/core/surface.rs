use ash::khr::surface;
use ash::khr::wayland_surface;
use ash::khr::win32_surface;
use ash::vk::{HINSTANCE, HWND};
use ash::{vk, Entry, Instance};
use winit::raw_window_handle::{RawDisplayHandle, RawWindowHandle, };

pub unsafe fn create_surface(
    entry: &Entry,
    instance: &Instance,
    raw_display_handle: RawDisplayHandle,
    raw_window_handle: RawWindowHandle,
) -> (surface::Instance, vk::SurfaceKHR) {
    let surface = match (raw_display_handle, raw_window_handle) {
        (RawDisplayHandle::Windows(_), RawWindowHandle::Win32(window)) => {
            let create_info = vk::Win32SurfaceCreateInfoKHR::default()
                .hinstance(window.hinstance.unwrap().get() as HINSTANCE)
                .hwnd(window.hwnd.get() as HWND);
            let loader = win32_surface::Instance::new(entry, instance);
            loader.create_win32_surface(&create_info, None).unwrap()
        }
        (RawDisplayHandle::Wayland(display), RawWindowHandle::Wayland(window)) => {
            let surface_desc = vk::WaylandSurfaceCreateInfoKHR::default()
                .display(display.display.as_ptr())
                .surface(window.surface.as_ptr());
            let loader = wayland_surface::Instance::new(entry, instance);
            loader.create_wayland_surface(&surface_desc, None).unwrap()
        }
        _ => {
            panic!("surface creation not supported for platform")
        }
    };
    (surface::Instance::new(entry, instance), surface)
}
