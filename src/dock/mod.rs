use std::sync::{mpsc::Sender, Arc};

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{WindowAttributes, WindowId, Window},
};

use self::render::{start_render_thread, RenderUpdate};

mod render;

#[derive(Default)]
pub struct Dock {
    window: Option<Arc<Window>>,
    render_thread: Option<Sender<RenderUpdate>>,
}

impl ApplicationHandler for Dock {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.render_thread.is_none() {
            let attributes = WindowAttributes::default().with_title("kew");
            let window = Arc::new(event_loop.create_window(attributes).unwrap());
            self.render_thread = Some(start_render_thread(window.clone()));
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
