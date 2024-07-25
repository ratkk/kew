use ash::vk;
use std::sync::{mpsc::Sender};

use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

use self::render::{start_render_thread, RenderUpdate};

mod render;

#[derive(Default)]
pub struct Dock {
    window: Option<Window>,
    render_thread: Option<Sender<RenderUpdate>>,
}

impl ApplicationHandler for Dock {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.render_thread.is_none() {
            let attributes = Window::default_attributes()
                .with_title("Kew Dock")
                .with_active(true);
            let window = event_loop.create_window(attributes).unwrap();

            let h_handle = window.display_handle().unwrap().as_raw();
            let w_handle = window.window_handle().unwrap().as_raw();

            self.render_thread = Some(start_render_thread(
                h_handle,
                w_handle,
                get_window_extent(&window),
            ));
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
                if let Some(sender) = &self.render_thread {
                    sender.send(RenderUpdate::REDRAW).unwrap();
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
