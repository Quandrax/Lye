use winit::event_loop::EventLoop;

mod camera;
mod setup;

fn main() {
    let event_loop = EventLoop::new().expect("Why would this fail");
    let mut app = setup::VulkanRenderer::new();

    event_loop.run_app(&mut app).expect("do be not working");

    drop(app);
}
