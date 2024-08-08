use ash::{
    prelude::VkResult,
    vk::{self},
};
use std::{ffi::CStr, ptr};
use winit::{
    self,
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::{Window, WindowAttributes},
};

pub(crate) struct IHateWinitVer30 {
    pub(crate) game: Option<VulkanRenderer>,
}
pub(crate) struct VulkanRenderer {
    window: Window,
    instance: ash::Instance,
    physical_device: vk::PhysicalDevice,
    device: ash::Device,
}

impl VulkanRenderer {
    fn new(event_loop: &ActiveEventLoop) -> Self {
        let window = event_loop
            .create_window(
                WindowAttributes::default()
                    .with_title("Lye")
                    .with_maximized(true)
                    .with_resizable(false),
            )
            .expect("no window");

        let entry = unsafe { ash::Entry::load() }.expect("No entry");
        let instance = VulkanRenderer::create_instance(&entry).expect("no instance");
        let surface =
            VulkanRenderer::create_surface(&window, &entry, &instance).expect("No surface");
        let physical_device =
            VulkanRenderer::get_physical_device(&instance).expect("No matching physical device");
        let device = VulkanRenderer::create_device(&instance, physical_device).expect("No device");

        Self {
            window,
            instance,
            physical_device,
            device,
        }
    }

    fn create_instance(entry: &ash::Entry) -> VkResult<ash::Instance> {
        let application_info = unsafe {
            vk::ApplicationInfo::default()
                .application_name(CStr::from_bytes_with_nul_unchecked(b"Lye\0"))
                .application_version(0)
                .engine_name(CStr::from_bytes_with_nul_unchecked(b"Fortnite-Engine\0"))
                .engine_version(0)
                .api_version(vk::make_api_version(0, 1, 3, 0))
        };
        //Dont like this StructType::default() thing, so wont continue using this

        let pp_enabled_extension_names = [
            ash::khr::surface::NAME.as_ptr(),
            ash::khr::win32_surface::NAME.as_ptr(),
        ];

        let create_info = vk::InstanceCreateInfo {
            s_type: vk::StructureType::INSTANCE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::InstanceCreateFlags::empty(),
            p_application_info: &application_info,
            enabled_layer_count: 0,
            pp_enabled_layer_names: ptr::null(),
            enabled_extension_count: pp_enabled_extension_names.len() as u32,
            pp_enabled_extension_names: pp_enabled_extension_names.as_ptr(),
            _marker: Default::default(),
        };

        unsafe { entry.create_instance(&create_info, None) }
    }

    fn create_surface(
        window: &Window,
        entry: &ash::Entry,
        instance: &ash::Instance,
    ) -> Result<vk::SurfaceKHR, vk::Result> {
        #[cfg(target_os = "windows")]
        let (hwnd, hinstance) = match window.window_handle().unwrap().as_raw() {
            RawWindowHandle::Win32(handle) => (handle.hwnd.get(), handle.hinstance.unwrap().get()),
            _ => {
                println!("Heheheha");
                panic!()
            }
        }; //Reminder 1 to change in the future

        let win32_surface_loader = ash::khr::win32_surface::Instance::new(entry, instance);
        let create_info = vk::Win32SurfaceCreateInfoKHR {
            s_type: vk::StructureType::WIN32_SURFACE_CREATE_INFO_KHR,
            p_next: ptr::null(),
            flags: vk::Win32SurfaceCreateFlagsKHR::empty(),
            hinstance,
            hwnd,
            _marker: Default::default(),
        };
        unsafe { win32_surface_loader.create_win32_surface(&create_info, None) }
    }

    fn get_physical_device(instance: &ash::Instance) -> Result<vk::PhysicalDevice, ()> {
        let physical_devices =
            unsafe { instance.enumerate_physical_devices() }.expect("No physical device found");

        println!("{} Vulkan device/s", physical_devices.len());

        let mut result = Err(());

        if physical_devices.len() == 1 {
            //Reminder 2 to change this some day so it works on every pc
            unsafe {
                match instance
                    .get_physical_device_properties(physical_devices[0])
                    .device_type
                {
                    vk::PhysicalDeviceType::CPU => println!("Picked CPU"),
                    vk::PhysicalDeviceType::INTEGRATED_GPU => println!("Picked Integrated GPU"),
                    vk::PhysicalDeviceType::DISCRETE_GPU => println!("Picked Discrete GPU"),
                    vk::PhysicalDeviceType::VIRTUAL_GPU => println!("Picked Virtual GPU"),
                    vk::PhysicalDeviceType::OTHER => println!("I dont know what was picked"),
                    _ => {
                        println!("Just crash at this point");
                        panic!();
                    }
                }
            }
            result = Ok(physical_devices[0]);
        }

        result
    }

    fn create_device(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
    ) -> VkResult<ash::Device> {
        let queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        let mut queue_family_indices = None;

        let mut index = 0;
        for queue_family in queue_families.iter() {
            if queue_family.queue_count > 0
                && queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS)
            {
                queue_family_indices = Some(index);
            }

            if queue_family_indices.is_some() {
                break;
            }

            index += 1;
        }

        let queue_priorities = [1.0];
        let queue_create_info = vk::DeviceQueueCreateInfo {
            s_type: vk::StructureType::DEVICE_QUEUE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::DeviceQueueCreateFlags::empty(),
            queue_family_index: queue_family_indices.unwrap(),
            queue_count: queue_priorities.len() as u32,
            p_queue_priorities: queue_priorities.as_ptr(),
            _marker: Default::default(),
        };

        let extension_names = [ash::khr::swapchain::NAME.as_ptr()];

        let create_info = vk::DeviceCreateInfo {
            s_type: vk::StructureType::DEVICE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::DeviceCreateFlags::empty(),
            queue_create_info_count: 1,
            p_queue_create_infos: &queue_create_info,
            enabled_extension_count: extension_names.len() as u32,
            pp_enabled_extension_names: extension_names.as_ptr(),
            p_enabled_features: ptr::null(),
            ..Default::default()
        };

        unsafe { instance.create_device(physical_device, &create_info, None) }
    }
}

impl ApplicationHandler for IHateWinitVer30 {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.game.is_none() {
            self.game = Some(VulkanRenderer::new(event_loop));
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
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
        /*unsafe {
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        };*/
    } //Getting Status_Access_Violation error using this, reminder 3 to fix it one day
}
