use ash::vk::{self};
use std::{ffi::CStr, ptr};
use winit::{
    self,
    application::ApplicationHandler,
    event::WindowEvent,
    window::{Window, WindowAttributes},
};

pub struct VulkanRenderer {
    window: Option<Window>,
    instance: ash::Instance,
    physical_device: vk::PhysicalDevice,
}

impl VulkanRenderer {
    pub fn new() -> Self {
        let entry = unsafe { ash::Entry::load() }.unwrap();
        let instance = VulkanRenderer::create_instance(&entry);
        let physical_device = VulkanRenderer::get_physical_device(&instance);

        Self {
            window: None,
            instance,
            physical_device,
        }
    }

    fn create_instance(entry: &ash::Entry) -> ash::Instance {
        let application_info = unsafe {
            vk::ApplicationInfo {
                s_type: vk::StructureType::APPLICATION_INFO,
                p_next: ptr::null(),
                p_application_name: CStr::from_bytes_with_nul_unchecked(b"Lye\0").as_ptr(),
                application_version: 0,
                p_engine_name: CStr::from_bytes_with_nul_unchecked(b"Fortnite-Engine\0").as_ptr(),
                engine_version: 0,
                api_version: vk::make_api_version(0, 1, 3, 0),
                _marker: Default::default(),
            }
        };

        let create_info = vk::InstanceCreateInfo {
            s_type: vk::StructureType::INSTANCE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::InstanceCreateFlags::empty(),
            p_application_info: &application_info,
            enabled_layer_count: 0,
            pp_enabled_layer_names: ptr::null(),
            enabled_extension_count: 0,
            pp_enabled_extension_names: ptr::null(),
            _marker: Default::default(),
        };

        unsafe { entry.create_instance(&create_info, None) }.unwrap()
    }

    fn get_physical_device(instance: &ash::Instance) -> vk::PhysicalDevice {
        let physical_devices = unsafe { instance.enumerate_physical_devices() }.unwrap();
        let physical_device = physical_devices[0];
        physical_device
    }
}

impl ApplicationHandler for VulkanRenderer {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.window = Some(
            event_loop
                .create_window(
                    WindowAttributes::default()
                        .with_title("Lye")
                        .with_maximized(true)
                        .with_resizable(false),
                )
                .expect("no window"),
        );
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

impl Drop for VulkanRenderer {
    fn drop(&mut self) {
        unsafe {
            self.instance.destroy_instance(None);
        };
    }
}
