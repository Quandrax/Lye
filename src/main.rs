use winit::event_loop::EventLoop;

mod setup;

fn main() {
    let event_loop = EventLoop::new().expect("No event loop");
    let mut app = setup::VulkanRenderer::new();

    event_loop.run_app(&mut app).expect("do be not working");
}
