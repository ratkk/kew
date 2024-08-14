use dock::Dock;
use winit::event_loop::{ControlFlow, EventLoop};

mod core;
mod dock;
mod math;

fn main() {
    env_logger::init();

    // let mut input = String::new();
    // io::stdin()
    //     .read_line(&mut input)
    //     .expect("failed to read from stdin");
    //
    // let input = input.trim();
    // match input.parse::<u32>() {
    //     Ok(0) => {
    //         let event_loop = EventLoop::new().unwrap();
    //         event_loop.set_control_flow(ControlFlow::Poll);
    //         let mut dock = Dock::default();
    //         let _ = event_loop.run_app(&mut dock);
    //     }
    //     Ok(i) => println!("program idx not found: {i}"),
    //     Err(..) => println!("failed to parse: {}", input),
    // };
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut dock = Dock::default();
    let _ = event_loop.run_app(&mut dock);
}
