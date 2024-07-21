use ash::khr::surface;
use ash::khr::wayland_surface;
use ash::khr::win32_surface;
use ash::vk::{HINSTANCE, HWND};
use ash::{vk, Entry, Instance};
use winit::raw_window_handle::{
    HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle,
};
use winit::window::Window;

pub unsafe fn create_surface(
    entry: &Entry,
    instance: &Instance,
    window: &Window,
) -> (surface::Instance, vk::SurfaceKHR) {
    let display_handle = window.display_handle().unwrap();
    let window_handle = window.window_handle().unwrap();

    let surface = match (display_handle.as_raw(), window_handle.as_raw()) {
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
