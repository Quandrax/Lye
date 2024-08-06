use winit::event_loop::EventLoop;

mod camera;
mod setup;

fn main() {
    let event_loop = EventLoop::new().expect("Why would this fail");
    let mut app = setup::IHateWinitVer30 { game: None };

    event_loop.run_app(&mut app).expect("do be not working");
}
