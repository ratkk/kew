use std::thread;
use ash::vk;
use crossbeam::channel::{Sender, unbounded};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};
use crate::core::context::KewContext;
use crate::core::device::{KewDevice, KewQueueIndices};
use crate::dock::dock::init_dock;

mod config;
mod dock;

pub enum DockMessage {
    TEST
}

pub enum DockErr {
    SOFT
}

#[derive(Default)]
pub struct Dock {
    window: Option<Window>,
    vk_thread: Option<Sender<DockMessage>>
}

impl ApplicationHandler for Dock {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.vk_thread.is_none() {
            let attributes = Window::default_attributes()
                .with_title("Kew Dock")
                .with_active(true);
            let window = event_loop.create_window(attributes).unwrap();

            let kew_context = KewContext::new();
            let (surface_loader, surface) = unsafe {
                crate::core::surface::create_surface(
                    &kew_context.entry,
                    &kew_context.instance,
                    window.display_handle().unwrap().as_raw(),
                    window.window_handle().unwrap().as_raw(),
                )
            };
            let window_extent = get_window_extent(&window);
            let queue_indices = KewQueueIndices::new(&kew_context, &surface_loader, surface);
            let kew_device = KewDevice::new(kew_context, &queue_indices);

            let (tx, rx) = unbounded();
            thread::spawn(move || {
                init_dock(
                    &kew_device,
                    &surface_loader,
                    surface,
                    &queue_indices,
                    window_extent,
                    rx
                );
            });
            self.vk_thread = Some(tx);
            self.window = Some(window);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                if let Some(sender) = &self.vk_thread {
                    sender.send(DockMessage::TEST).unwrap();
                }
            }
            _ => (),
        }
    }
}

fn get_window_extent(window: &Window) -> vk::Extent2D {
    let inner_size = window.inner_size();
    vk::Extent2D {
        width: inner_size.width,
        height: inner_size.height,
    }
}
