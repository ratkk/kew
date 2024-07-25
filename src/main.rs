use core::{
    command::KewCommandPool,
    context::KewContext,
    device::{KewDevice, KewQueueIndices},
};
use std::io;

use apps::{img::img_compute, sqr::sqr_compute};
use dock::Dock;
use winit::event_loop::{ControlFlow, EventLoop};

mod apps;
mod core;
mod dock;
mod math;

fn main() {
    env_logger::init();

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("failed to read from stdin");

    let input = input.trim();
    match input.parse::<u32>() {
        Ok(0) => {
            let event_loop = EventLoop::new().unwrap();
            event_loop.set_control_flow(ControlFlow::Poll);
            let mut dock = Dock::default();
            let _ = event_loop.run_app(&mut dock);
        }
        Ok(i @ 1..=2) => {
            let kew_context = KewContext::new();
            let queue_indices = KewQueueIndices::new(&kew_context);
            let device = KewDevice::new(kew_context, &queue_indices);

            let cmp_cmd_pool = KewCommandPool::new(&device, queue_indices.cmp_idx);
            let tfr_cmd_pool = KewCommandPool::new(&device, queue_indices.tfr_idx);
            match i {
                1 => sqr_compute(&device, &cmp_cmd_pool, &[-5, 10, -4]),
                2 => img_compute(&device, &cmp_cmd_pool, &tfr_cmd_pool),
                _ => {}
            }
        }
        Ok(i) => println!("program idx not found: {i}"),
        Err(..) => println!("failed to parse: {}", input),
    };
}
