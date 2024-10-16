use winit::event_loop::EventLoop;

mod camera;
mod setup;

fn main() {
    let event_loop = EventLoop::new().expect("Why would this fail");
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let mut app = setup::App::default();

    event_loop.run_app(&mut app).expect("do be not working");
}
