use ash::vk;
use std::ffi::CStr;
use winit::{
    self,
    application::ApplicationHandler,
    event::WindowEvent,
    window::{Window, WindowAttributes},
};

pub struct VulkanRenderer {
    window: Option<Window>,
}

impl VulkanRenderer {
    pub fn new() -> Self {
        Self { window: None }
    }

    fn create_instance(entry: &ash::Entry) -> ash::Instance {
        let application_info = unsafe {
            vk::ApplicationInfo {
                p_application_name: CStr::from_bytes_with_nul_unchecked(b"Lye\0").as_ptr(),
                api_version: vk::make_api_version(0, 1, 3, 0),
                ..Default::default()
            }
        };
        let create_info = vk::InstanceCreateInfo {
            p_application_info: &application_info,
            ..Default::default()
        };
        unsafe { entry.create_instance(&create_info, None) }.unwrap()
    }

    fn get_physical_device(instance: &ash::Instance) {
        let physical_devices = unsafe { instance.enumerate_physical_devices() }.unwrap();

        println!("Found {} that support Vulkan", physical_devices.len());
    }
}

impl ApplicationHandler for VulkanRenderer {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.window = Some(
            event_loop
                .create_window(
                    WindowAttributes::default()
                        .with_title("Lye")
                        .with_maximized(true),
                )
                .expect("no window"),
        );

        let entry = unsafe { ash::Entry::load() }.unwrap();

        let instance = VulkanRenderer::create_instance(&entry);
        let physical_device = VulkanRenderer::get_physical_device(&instance);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => (),
            _ => (),
        }
    }
}
