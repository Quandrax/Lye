use ash::{
    prelude::VkResult,
    vk::{self},
};
use core::panic;
use std::{ffi::CStr, ptr};
use winit::{
    self,
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::{Window, WindowAttributes},
};

#[derive(Default)]
pub(crate) struct IHateWinitVer30 {
    game: Option<VulkanRenderer>,
}

struct VulkanRenderer {
    window: Window,
    instance: ash::Instance,
    surface: vk::SurfaceKHR,
    surface_loader: ash::khr::surface::Instance,
    physical_device: vk::PhysicalDevice,
    device: ash::Device,
    graphics_queue: vk::Queue,
    gq_family_index: u32,
    swapchain_loader: ash::khr::swapchain::Device,
    swapchain: vk::SwapchainKHR,
}

impl VulkanRenderer {
    fn new(event_loop: &ActiveEventLoop) -> Result<Self, vk::Result> {
        let window = event_loop
            .create_window(
                WindowAttributes::default()
                    .with_title("Lye")
                    .with_maximized(true)
                    .with_resizable(false),
            )
            .expect("no window");

        let entry = unsafe { ash::Entry::load() }.expect("Entry no work");
        let instance = VulkanRenderer::create_instance(&entry)?;
        let surface_loader = ash::khr::surface::Instance::new(&entry, &instance);
        let surface = VulkanRenderer::create_surface(&window, &entry, &instance)?;
        let physical_device = VulkanRenderer::get_physical_device(&instance)?;
        let (device, graphics_queue, gq_family_index) =
            VulkanRenderer::create_device(&instance, physical_device)?;
        let (swapchain, swapchain_loader) = VulkanRenderer::create_swapchain(
            &instance,
            &surface_loader,
            physical_device,
            gq_family_index,
            surface,
            &device,
        )?;

        Ok(Self {
            window,
            instance,
            surface,
            surface_loader,
            physical_device,
            device,
            graphics_queue,
            gq_family_index,
            swapchain_loader,
            swapchain,
        })
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

        let extension_names = [
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
            enabled_extension_count: extension_names.len() as u32,
            pp_enabled_extension_names: extension_names.as_ptr(),
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
        {
            let (hwnd, hinstance) = match window.window_handle().unwrap().as_raw() {
                RawWindowHandle::Win32(handle) => {
                    (handle.hwnd.get(), handle.hinstance.unwrap().get())
                }
                _ => panic!(),
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

        #[cfg(target_os = "linux")]
        {
            todo!()
        }

        #[cfg(target_os = "macos")]
        {
            todo!()
        }
    }

    fn get_physical_device(instance: &ash::Instance) -> Result<vk::PhysicalDevice, vk::Result> {
        let physical_devices = unsafe { instance.enumerate_physical_devices() }.unwrap();

        let mut result = Err(vk::Result::INCOMPLETE);

        if physical_devices.is_empty() {
            return result;
        }

        println!("{} Vulkan device/s", physical_devices.len());

        if physical_devices.len() == 1 {
            //Reminder 2 to change this some day so it works on every pc
            unsafe {
                match instance
                    .get_physical_device_properties(physical_devices[0])
                    .device_type
                {
                    vk::PhysicalDeviceType::DISCRETE_GPU => println!("Picked Discrete GPU"),
                    vk::PhysicalDeviceType::INTEGRATED_GPU => println!("Picked Integrated GPU"),
                    vk::PhysicalDeviceType::VIRTUAL_GPU => println!("Picked Virtual GPU"),
                    vk::PhysicalDeviceType::CPU => println!("Picked CPU"),
                    vk::PhysicalDeviceType::OTHER => println!("I dont know what was picked"),
                    _ => {
                        println!("Just crash at this point");
                        panic!();
                    }
                }
            }
            result = Ok(physical_devices[0]);
        }
        unsafe {
            println!(
                "name : {:?}",
                instance
                    .get_physical_device_properties(result.unwrap())
                    .device_name_as_c_str()
                    .unwrap()
            )
        };

        result
    }

    fn create_device(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
    ) -> VkResult<(ash::Device, vk::Queue, u32)> {
        let queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        for q in &queue_families {
            println!("Q count {}, Q flags {:?}", q.queue_count, q.queue_flags);
        }

        let mut queue_family_index = None;

        for (index, queue_family) in queue_families.iter().enumerate() {
            if queue_family.queue_count > 0
                && queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS)
            {
                queue_family_index = Some(index as u32);
                println!("Index of q {}", index)
            }

            if queue_family_index.is_some() {
                break;
            }
        }

        let queue_priorities = [1.0];
        let queue_create_info = vk::DeviceQueueCreateInfo {
            s_type: vk::StructureType::DEVICE_QUEUE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::DeviceQueueCreateFlags::empty(),
            queue_family_index: queue_family_index.unwrap(),
            queue_count: 1,
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

        let device = unsafe { instance.create_device(physical_device, &create_info, None) }?;
        let queue = unsafe { device.get_device_queue(queue_family_index.unwrap(), 0) };

        Ok((device, queue, queue_family_index.unwrap()))
    }

    fn create_swapchain(
        instance: &ash::Instance,
        surface_loader: &ash::khr::surface::Instance,
        physical_device: vk::PhysicalDevice,
        queue_family_index: u32,
        surface: vk::SurfaceKHR,
        device: &ash::Device,
    ) -> VkResult<(vk::SwapchainKHR, ash::khr::swapchain::Device)> {
        unsafe {
            if !surface_loader
                .get_physical_device_surface_support(physical_device, queue_family_index, surface)
                .unwrap()
            {
                return Err(vk::Result::INCOMPLETE);
            }
        };

        let image_format = unsafe {
            surface_loader
                .get_physical_device_surface_formats(physical_device, surface)
                .unwrap()[0]
        };

        let img_resolution = unsafe {
            match surface_loader
                .get_physical_device_surface_capabilities(physical_device, surface)
                .unwrap()
                .current_extent
                .width
            {
                u32::MAX => panic!(),
                _ => {
                    let y = surface_loader
                        .get_physical_device_surface_capabilities(physical_device, surface)
                        .unwrap()
                        .current_extent;
                    println!("Extent : {:?}", y);
                    y
                }
            }
        };

        unsafe {
            println!(
                "Min image count {}",
                surface_loader
                    .get_physical_device_surface_capabilities(physical_device, surface)
                    .unwrap()
                    .min_image_count
            )
        };

        let swapchain_loader = ash::khr::swapchain::Device::new(instance, device);
        let swapchain_create_info = vk::SwapchainCreateInfoKHR {
            s_type: vk::StructureType::SWAPCHAIN_CREATE_INFO_KHR,
            p_next: ptr::null(),
            flags: vk::SwapchainCreateFlagsKHR::empty(),
            surface,
            min_image_count: 1,
            image_format: image_format.format,
            image_color_space: image_format.color_space,
            image_extent: img_resolution,
            image_array_layers: 1,
            image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
            image_sharing_mode: vk::SharingMode::EXCLUSIVE,
            pre_transform: vk::SurfaceTransformFlagsKHR::IDENTITY,
            composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
            present_mode: vk::PresentModeKHR::FIFO,
            clipped: vk::FALSE,
            queue_family_index_count: Default::default(),
            p_queue_family_indices: ptr::null(),
            old_swapchain: Default::default(),
            _marker: Default::default(),
        };

        let swapchain = unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None) }?;
        Ok((swapchain, swapchain_loader))
    }
}

impl ApplicationHandler for IHateWinitVer30 {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.game.is_none() {
            self.game = Some(VulkanRenderer::new(event_loop).unwrap());
        }
    }

    #[inline]
    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => println!("Redraw requested"),
            _ => (),
        }
    }
}

impl Drop for VulkanRenderer {
    fn drop(&mut self) {
        unsafe {
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.instance.destroy_instance(None);
        };
    } //Getting Status_Access_Violation error using this, reminder 3 to fix it one day
}
