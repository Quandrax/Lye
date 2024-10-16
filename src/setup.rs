use ash::{self, prelude::VkResult, vk};
use std::{marker::PhantomData, ptr};
use winit::{self, raw_window_handle::HasWindowHandle};

unsafe extern "system" fn vulkan_debug_callback(
    flag: vk::DebugUtilsMessageSeverityFlagsEXT,
    typ: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    use vk::DebugUtilsMessageSeverityFlagsEXT as Flag;

    let message = std::ffi::CStr::from_ptr((*p_callback_data).p_message);
    match flag {
        Flag::VERBOSE => println!("{:?} - {:?}", typ, message),
        Flag::INFO => println!("{:?} - {:?}", typ, message),
        Flag::WARNING => println!("{:?} - {:?}", typ, message),
        _ => println!("{:?} - {:?}", typ, message),
    }
    vk::FALSE
}

#[derive(Default)]
pub struct App {
    renderer: Option<Renderer>,
}

struct Renderer {
    window: winit::window::Window,
    instance: ash::Instance,
    debug_utils: ash::ext::debug_utils::Instance,
    debug_utils_messenger: vk::DebugUtilsMessengerEXT,
    surface_loader: ash::khr::surface::Instance,
    surface: vk::SurfaceKHR,
    physical_device: vk::PhysicalDevice,
    queue_family_index: usize,
    device: ash::Device,
    present_graphics_queue: vk::Queue,
    swapchain_loader: ash::khr::swapchain::Device,
    swapchain: vk::SwapchainKHR,
    format: vk::Format,
    extent: vk::Extent2D,
    images: Vec<vk::Image>,
    image_views: Vec<vk::ImageView>,
    render_pass: vk::RenderPass,
    framebuffers: Vec<vk::Framebuffer>,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    image_available: Vec<vk::Semaphore>,
    rendering_finished: Vec<vk::Semaphore>,
    can_draw: Vec<vk::Fence>,
    current_image: usize,
}

impl Renderer {
    fn new(
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let window = event_loop.create_window(
            winit::window::WindowAttributes::default()
                .with_title("Lye")
                .with_maximized(true)
                .with_resizable(false),
        )?;

        let entry = unsafe { ash::Entry::load() }?;
        let instance = Renderer::create_instance(&entry)?;
        let (debug_utils, debug_utils_messenger) = Renderer::debug_utils(&entry, &instance)?;
        let surface_loader = ash::khr::surface::Instance::new(&entry, &instance);
        let surface = Renderer::create_surface(&window, &entry, &instance)?;
        let (physical_device, queue_family_index) =
            Renderer::get_physical_device_and_queue_family(&instance, &surface_loader, surface)?;
        let (device, queue) =
            Renderer::create_device_and_queues(queue_family_index, &instance, physical_device)?;
        let (swapchain, swapchain_loader, format, extent) = Renderer::create_swapchain(
            &surface_loader,
            physical_device,
            surface,
            &instance,
            &device,
            queue_family_index,
        )?;
        let (images, image_views) = Renderer::acquire_swapchain_images_and_image_views(
            &swapchain_loader,
            swapchain,
            format,
            &device,
        )?;
        let render_pass = Renderer::create_renderpass(format, &device)?;
        let framebuffers =
            Renderer::create_framebuffers(&device, render_pass, &image_views, extent)?;
        let (pipeline, pipeline_layout) = Renderer::create_pipeline(&device, extent, render_pass)?;
        let (command_pool, command_buffers) =
            Renderer::create_command_buffers(queue_family_index, &device, images.len())?;
        let (image_available, rendering_finished, can_draw) =
            Renderer::create_semaphores_and_fences(images.len(), &device)?;
        Renderer::record_command_buffer(
            &command_buffers,
            &device,
            render_pass,
            &framebuffers,
            extent,
            pipeline,
        )?;

        Ok(Self {
            window,
            instance,
            debug_utils,
            debug_utils_messenger,
            surface_loader,
            surface,
            queue_family_index,
            physical_device,
            device,
            present_graphics_queue: queue,
            swapchain_loader,
            swapchain,
            format,
            extent,
            images,
            image_views,
            render_pass,
            framebuffers,
            pipeline_layout,
            pipeline,
            command_pool,
            command_buffers,
            image_available,
            rendering_finished,
            can_draw,
            current_image: 0,
        })
    }

    fn create_instance(entry: &ash::Entry) -> Result<ash::Instance, vk::Result> {
        let (major, minor, patch) = match unsafe {
            entry
                .try_enumerate_instance_version()
                .unwrap_or_else(|err| panic!("Error : {}", err))
        } {
            Some(version) => (
                vk::api_version_major(version),
                vk::api_version_minor(version),
                vk::api_version_patch(version),
            ),
            None => (1, 0, 0),
        };

        println!("Api ver : {}.{}.{}", major, minor, patch);

        let app_name = unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"Lye\0") };
        let engine_name =
            unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"Fortnite-Engine\0") };
        let app_info = vk::ApplicationInfo {
            s_type: vk::StructureType::APPLICATION_INFO,
            p_next: ptr::null(),
            p_application_name: app_name.as_ptr(),
            application_version: vk::make_api_version(0, 0, 1, 0),
            p_engine_name: engine_name.as_ptr(),
            engine_version: vk::make_api_version(0, 0, 1, 0),
            api_version: vk::make_api_version(0, major, minor, 0),
            _marker: PhantomData,
        };

        let extensions = [
            ash::khr::surface::NAME.as_ptr(),
            ash::khr::win32_surface::NAME.as_ptr(),
            ash::ext::debug_utils::NAME.as_ptr(),
        ];

        #[cfg(target_os = "macos")]
        {
            panic!() //LOL
        }

        let create_info = vk::InstanceCreateInfo {
            s_type: vk::StructureType::INSTANCE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::InstanceCreateFlags::empty(),
            p_application_info: &app_info,
            enabled_layer_count: 0,
            pp_enabled_layer_names: ptr::null(),
            enabled_extension_count: extensions.len() as u32,
            pp_enabled_extension_names: extensions.as_ptr(),
            _marker: PhantomData,
        };

        unsafe { entry.create_instance(&create_info, None) }
    }

    fn debug_utils(
        entry: &ash::Entry,
        instance: &ash::Instance,
    ) -> VkResult<(ash::ext::debug_utils::Instance, vk::DebugUtilsMessengerEXT)> {
        let message_severity = vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
            | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
            | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
            | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR;
        let message_type = vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
            | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
            | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION;

        let create_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(message_severity)
            .message_type(message_type)
            .pfn_user_callback(Some(vulkan_debug_callback));

        let debug_utils = ash::ext::debug_utils::Instance::new(entry, instance);
        let debug_utils_messenger =
            unsafe { debug_utils.create_debug_utils_messenger(&create_info, None)? };

        Ok((debug_utils, debug_utils_messenger))
    }

    fn create_surface(
        window: &winit::window::Window,
        entry: &ash::Entry,
        instance: &ash::Instance,
    ) -> VkResult<vk::SurfaceKHR> {
        #[cfg(target_os = "windows")]
        {
            let (hwnd, hinstance) = match window.window_handle().unwrap().as_raw() {
                winit::raw_window_handle::RawWindowHandle::Win32(handle) => {
                    (handle.hwnd.get(), handle.hinstance.unwrap().get())
                }
                _ => panic!("I dont know, surface wrong i guess"),
            };

            let create_info = vk::Win32SurfaceCreateInfoKHR {
                s_type: vk::StructureType::WIN32_SURFACE_CREATE_INFO_KHR,
                p_next: ptr::null(),
                flags: vk::Win32SurfaceCreateFlagsKHR::empty(),
                hinstance,
                hwnd,
                _marker: PhantomData,
            };

            let win_surface_loader = ash::khr::win32_surface::Instance::new(entry, instance);
            unsafe { win_surface_loader.create_win32_surface(&create_info, None) }
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

    fn get_physical_device_and_queue_family(
        instance: &ash::Instance,
        surface_loader: &ash::khr::surface::Instance,
        surface: vk::SurfaceKHR,
    ) -> Result<(vk::PhysicalDevice, usize), vk::Result> {
        let physical_devices = unsafe { instance.enumerate_physical_devices() }?;

        println!("{} Vulkan device/s", physical_devices.len());

        let (physical_device, queue_family_index) = unsafe {
            physical_devices.iter().find_map(|p_device| {
                instance
                    .get_physical_device_queue_family_properties(*p_device)
                    .iter()
                    .enumerate()
                    .find_map(|(i, properties)| {
                        let supp_surface_and_graphics = properties
                            .queue_flags
                            .contains(vk::QueueFlags::GRAPHICS)
                            && surface_loader
                                .get_physical_device_surface_support(*p_device, i as u32, surface)
                                .unwrap();
                        if supp_surface_and_graphics {
                            let prop = instance.get_physical_device_properties(*p_device);
                            println!(
                                "Device Type {:?}, Device Name {:?}",
                                prop.device_type,
                                prop.device_name_as_c_str().unwrap()
                            );
                            Some((*p_device, i))
                        } else {
                            None
                        }
                    })
            })
        }
        .expect("Couldnt find suitable physical devices"); //Not sure if this selects the best pdevice, looks like it only selects the first one

        let properties = unsafe {
            instance.get_physical_device_queue_family_properties(physical_device)
                [queue_family_index]
        };

        println!(
            "Q count : {}, Q flags : {:?}",
            properties.queue_count, properties.queue_flags
        );

        Ok((physical_device, queue_family_index))
    }

    fn create_device_and_queues(
        queue_family_index: usize,
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
    ) -> VkResult<(ash::Device, vk::Queue)> {
        let queue_create_info = vk::DeviceQueueCreateInfo {
            s_type: vk::StructureType::DEVICE_QUEUE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::DeviceQueueCreateFlags::empty(),
            queue_family_index: queue_family_index as u32,
            queue_count: 1,
            p_queue_priorities: [1.0].as_ptr(),
            _marker: PhantomData,
        };

        let device_extensions = [ash::khr::swapchain::NAME.as_ptr()];

        let device_create_info = vk::DeviceCreateInfo {
            s_type: vk::StructureType::DEVICE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::DeviceCreateFlags::empty(),
            queue_create_info_count: 1,
            p_queue_create_infos: &queue_create_info,
            enabled_extension_count: device_extensions.len() as u32,
            pp_enabled_extension_names: device_extensions.as_ptr(),
            p_enabled_features: ptr::null(),
            _marker: PhantomData,
            ..Default::default()
        };

        let device = unsafe { instance.create_device(physical_device, &device_create_info, None) }?;
        let queue = unsafe { device.get_device_queue(queue_family_index as u32, 0) };

        Ok((device, queue))
    }

    fn create_swapchain(
        surface_loader: &ash::khr::surface::Instance,
        physical_device: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
        instance: &ash::Instance,
        device: &ash::Device,
        queue_family_index: usize,
    ) -> VkResult<(
        vk::SwapchainKHR,
        ash::khr::swapchain::Device,
        vk::Format,
        vk::Extent2D,
    )> {
        let image_format = unsafe {
            surface_loader.get_physical_device_surface_formats(physical_device, surface)?[0]
        };

        let image_resolution = unsafe {
            match surface_loader
                .get_physical_device_surface_capabilities(physical_device, surface)?
                .current_extent
                .width
            {
                u32::MAX => todo!(),
                _ => {
                    surface_loader
                        .get_physical_device_surface_capabilities(physical_device, surface)?
                        .current_extent
                }
            }
        };
        let queue_family_indeces = vec![queue_family_index as u32];

        let swapchain_create_info = vk::SwapchainCreateInfoKHR {
            s_type: vk::StructureType::SWAPCHAIN_CREATE_INFO_KHR,
            p_next: ptr::null(),
            flags: vk::SwapchainCreateFlagsKHR::empty(),
            surface,
            min_image_count: 3,
            image_format: image_format.format,
            image_color_space: image_format.color_space,
            image_extent: image_resolution,
            image_array_layers: 1,
            image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
            image_sharing_mode: vk::SharingMode::EXCLUSIVE,
            pre_transform: vk::SurfaceTransformFlagsKHR::IDENTITY,
            composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
            present_mode: vk::PresentModeKHR::FIFO,
            clipped: vk::FALSE,
            queue_family_index_count: 1,
            p_queue_family_indices: queue_family_indeces.as_ptr(),
            old_swapchain: vk::SwapchainKHR::null(),
            _marker: PhantomData,
        };
        let swapchain_loader = ash::khr::swapchain::Device::new(instance, device);
        let swapchain = unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None)? };

        Ok((
            swapchain,
            swapchain_loader,
            image_format.format,
            image_resolution,
        ))
    }

    fn acquire_swapchain_images_and_image_views(
        swapchain_loader: &ash::khr::swapchain::Device,
        swapchain: vk::SwapchainKHR,
        format: vk::Format,
        device: &ash::Device,
    ) -> VkResult<(Vec<vk::Image>, Vec<vk::ImageView>)> {
        let images = unsafe { swapchain_loader.get_swapchain_images(swapchain)? };
        let mut image_views = Vec::with_capacity(images.len());

        for image in images.iter() {
            let img_view_create_info = vk::ImageViewCreateInfo {
                s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
                p_next: ptr::null(),
                flags: vk::ImageViewCreateFlags::empty(),
                image: *image,
                view_type: vk::ImageViewType::TYPE_2D,
                format: format,
                components: vk::ComponentMapping {
                    r: vk::ComponentSwizzle::IDENTITY,
                    g: vk::ComponentSwizzle::IDENTITY,
                    b: vk::ComponentSwizzle::IDENTITY,
                    a: vk::ComponentSwizzle::IDENTITY,
                },
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                _marker: PhantomData,
            };

            image_views.push(unsafe { device.create_image_view(&img_view_create_info, None)? });
        }

        println!("Using {} images/image_views", images.len());

        Ok((images, image_views))
    }

    fn create_renderpass(format: vk::Format, device: &ash::Device) -> VkResult<vk::RenderPass> {
        let attachment_description = vk::AttachmentDescription {
            flags: vk::AttachmentDescriptionFlags::empty(),
            format,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
            stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
            stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
        };
        //How attachments are handled before/after renderpass

        let color_attachment_ref = vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        };
        //How attachments are used (layout) and which attachmentdescriptions are used for current subpass

        let subpass_description = vk::SubpassDescription {
            flags: vk::SubpassDescriptionFlags::empty(),
            pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
            color_attachment_count: 1,
            p_color_attachments: &color_attachment_ref,
            input_attachment_count: 0,
            p_input_attachments: ptr::null(),
            p_resolve_attachments: ptr::null(),
            p_depth_stencil_attachment: ptr::null(),
            preserve_attachment_count: 0,
            p_preserve_attachments: ptr::null(),
            _marker: PhantomData,
        };
        //Does Render-operations based on attachments

        let subpass_dependency = vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            dst_subpass: 0,
            src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            src_access_mask: vk::AccessFlags::empty(),
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_READ
                | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            dependency_flags: vk::DependencyFlags::empty(),
        };
        //Describes relation between subpasses

        let render_pass_create_info = vk::RenderPassCreateInfo {
            s_type: vk::StructureType::RENDER_PASS_CREATE_INFO,
            flags: vk::RenderPassCreateFlags::empty(),
            p_next: ptr::null(),
            attachment_count: 1,
            p_attachments: &attachment_description,
            subpass_count: 1,
            p_subpasses: &subpass_description,
            dependency_count: 1,
            p_dependencies: &subpass_dependency,
            _marker: PhantomData,
        };

        unsafe { device.create_render_pass(&render_pass_create_info, None) }
    }

    fn create_framebuffers(
        device: &ash::Device,
        render_pass: vk::RenderPass,
        image_views: &[vk::ImageView],
        extent: vk::Extent2D,
    ) -> VkResult<Vec<vk::Framebuffer>> {
        let mut framebuffers = Vec::with_capacity(image_views.len());

        for image in image_views {
            let framebuffer_create_info = vk::FramebufferCreateInfo {
                s_type: vk::StructureType::FRAMEBUFFER_CREATE_INFO,
                p_next: ptr::null(),
                flags: vk::FramebufferCreateFlags::empty(),
                render_pass,
                p_attachments: image,
                attachment_count: 1,
                width: extent.width,
                height: extent.height,
                layers: 1,
                _marker: Default::default(),
            };
            framebuffers
                .push(unsafe { device.create_framebuffer(&framebuffer_create_info, None)? });
        }

        Ok(framebuffers)
    }

    fn create_shader_module(bytes: &[u8], device: &ash::Device) -> VkResult<vk::ShaderModule> {
        let mut cursor = std::io::Cursor::new(bytes);
        let shader_bytes = ash::util::read_spv(&mut cursor).unwrap();
        let shader_create_info = vk::ShaderModuleCreateInfo::default().code(&shader_bytes);
        unsafe { device.create_shader_module(&shader_create_info, None) }
    }

    fn create_pipeline(
        device: &ash::Device,
        extent: vk::Extent2D,
        render_pass: vk::RenderPass,
    ) -> VkResult<(vk::Pipeline, vk::PipelineLayout)> {
        let vertex_shader_module =
            Renderer::create_shader_module(&include_bytes!("../shaders/vertex.spv")[..], device)?;
        let fragment_shader_module =
            Renderer::create_shader_module(&include_bytes!("../shaders/fragment.spv")[..], device)?;

        let entry = unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"main\0") }; //might be a problem

        let shader_states_create_infos = [
            vk::PipelineShaderStageCreateInfo {
                s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
                p_next: ptr::null(),
                flags: vk::PipelineShaderStageCreateFlags::empty(),
                stage: vk::ShaderStageFlags::VERTEX,
                module: vertex_shader_module,
                p_name: entry.as_ptr(),
                p_specialization_info: ptr::null(),
                _marker: PhantomData,
            },
            vk::PipelineShaderStageCreateInfo {
                s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
                p_next: ptr::null(),
                flags: vk::PipelineShaderStageCreateFlags::empty(),
                stage: vk::ShaderStageFlags::FRAGMENT,
                module: fragment_shader_module,
                p_name: entry.as_ptr(),
                p_specialization_info: ptr::null(),
                _marker: PhantomData,
            },
        ];

        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineVertexInputStateCreateFlags::empty(),
            vertex_attribute_description_count: 0,
            p_vertex_attribute_descriptions: ptr::null(),
            vertex_binding_description_count: 0,
            p_vertex_binding_descriptions: ptr::null(),
            _marker: PhantomData,
        };

        let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineInputAssemblyStateCreateFlags::empty(),
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            primitive_restart_enable: vk::FALSE,
            _marker: PhantomData,
        };
        //How vertices form a shape

        let viewports = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: extent.width as f32,
            height: extent.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];
        //Area rendered

        let scissors = [vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent,
        }];
        //Which part of viewport is rendered

        let viewport_state = vk::PipelineViewportStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_VIEWPORT_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineViewportStateCreateFlags::empty(),
            viewport_count: 1,
            p_viewports: viewports.as_ptr(),
            scissor_count: 1,
            p_scissors: scissors.as_ptr(),
            _marker: PhantomData,
        };

        let rasterization_state = vk::PipelineRasterizationStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_RASTERIZATION_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineRasterizationStateCreateFlags::empty(),
            depth_clamp_enable: vk::FALSE,
            rasterizer_discard_enable: vk::FALSE,
            polygon_mode: vk::PolygonMode::FILL,
            cull_mode: vk::CullModeFlags::BACK,
            front_face: vk::FrontFace::CLOCKWISE,
            depth_bias_enable: vk::FALSE,
            depth_bias_constant_factor: 0.0,
            depth_bias_clamp: 0.0,
            depth_bias_slope_factor: 0.0,
            line_width: 1.0,
            _marker: PhantomData,
        };
        //Coordinates to Pixels, face-culling (cut out none visible parts)

        let multisample_state = vk::PipelineMultisampleStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_MULTISAMPLE_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineMultisampleStateCreateFlags::empty(),
            rasterization_samples: vk::SampleCountFlags::TYPE_1,
            sample_shading_enable: vk::FALSE,
            min_sample_shading: 1.0,
            p_sample_mask: ptr::null(),
            alpha_to_coverage_enable: 0,
            alpha_to_one_enable: 0,
            _marker: PhantomData,
        };
        //Multisampling

        let color_blend_attachments = [vk::PipelineColorBlendAttachmentState {
            blend_enable: vk::FALSE,
            src_color_blend_factor: vk::BlendFactor::ONE,
            dst_color_blend_factor: vk::BlendFactor::ZERO,
            color_blend_op: vk::BlendOp::ADD,
            src_alpha_blend_factor: vk::BlendFactor::ONE,
            dst_alpha_blend_factor: vk::BlendFactor::ZERO,
            alpha_blend_op: vk::BlendOp::ADD,
            color_write_mask: vk::ColorComponentFlags::RGBA,
        }];

        let color_blend_state = vk::PipelineColorBlendStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_COLOR_BLEND_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineColorBlendStateCreateFlags::empty(),
            logic_op: vk::LogicOp::COPY,
            logic_op_enable: vk::FALSE,
            attachment_count: 1,
            p_attachments: color_blend_attachments.as_ptr(),
            blend_constants: [0.0, 0.0, 0.0, 0.0],
            _marker: PhantomData,
        };
        //Combines color from framebuffer and newly rendered color

        let pipeline_layout_info = vk::PipelineLayoutCreateInfo {
            s_type: vk::StructureType::PIPELINE_LAYOUT_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineLayoutCreateFlags::empty(),
            set_layout_count: 0,
            p_set_layouts: ptr::null(),
            push_constant_range_count: 0,
            p_push_constant_ranges: ptr::null(),
            _marker: PhantomData,
        };

        let pipeline_layout =
            unsafe { device.create_pipeline_layout(&pipeline_layout_info, None)? };

        let pipeline_create_info = [vk::GraphicsPipelineCreateInfo {
            s_type: vk::StructureType::GRAPHICS_PIPELINE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineCreateFlags::empty(),
            stage_count: shader_states_create_infos.len() as u32,
            p_stages: shader_states_create_infos.as_ptr(),
            p_vertex_input_state: &vertex_input_state,
            p_input_assembly_state: &input_assembly_state,
            p_tessellation_state: ptr::null(),
            p_viewport_state: &viewport_state,
            p_rasterization_state: &rasterization_state,
            p_multisample_state: &multisample_state,
            p_depth_stencil_state: ptr::null(),
            p_color_blend_state: &color_blend_state,
            p_dynamic_state: ptr::null(),
            layout: pipeline_layout,
            render_pass,
            subpass: 0,
            base_pipeline_handle: Default::default(),
            base_pipeline_index: 0,
            _marker: PhantomData,
        }];

        let pipeline = unsafe {
            device
                .create_graphics_pipelines(vk::PipelineCache::null(), &pipeline_create_info, None)
                .unwrap()[0]
        };

        unsafe {
            device.destroy_shader_module(vertex_shader_module, None);
            device.destroy_shader_module(fragment_shader_module, None);
        };

        Ok((pipeline, pipeline_layout))
    }

    fn create_command_buffers(
        queue_family_index: usize,
        device: &ash::Device,
        count: usize,
    ) -> VkResult<(vk::CommandPool, Vec<vk::CommandBuffer>)> {
        let commandpool_create_info = vk::CommandPoolCreateInfo {
            s_type: vk::StructureType::COMMAND_POOL_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::CommandPoolCreateFlags::TRANSIENT,
            queue_family_index: queue_family_index as u32,
            _marker: PhantomData,
        };

        let command_pool = unsafe { device.create_command_pool(&commandpool_create_info, None)? };

        let allocate_info = vk::CommandBufferAllocateInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
            p_next: ptr::null(),
            command_pool,
            level: vk::CommandBufferLevel::PRIMARY,
            command_buffer_count: count as u32,
            _marker: PhantomData,
        };

        let command_buffers = unsafe { device.allocate_command_buffers(&allocate_info)? };

        Ok((command_pool, command_buffers))
    }

    fn record_command_buffer(
        command_buffers: &[vk::CommandBuffer],
        device: &ash::Device,
        render_pass: vk::RenderPass,
        framebuffers: &[vk::Framebuffer],
        extent: vk::Extent2D,
        pipeline: vk::Pipeline,
    ) -> VkResult<()> {
        let begin_info = vk::CommandBufferBeginInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
            p_next: ptr::null(),
            flags: vk::CommandBufferUsageFlags::empty(),
            p_inheritance_info: ptr::null(),
            _marker: PhantomData,
        };

        let render_area = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent,
        };

        let clear_values = [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [1.0, 1.0, 1.0, 1.0],
            },
        }];

        for (i, &command_buffer) in command_buffers.iter().enumerate() {
            let render_pass_begin = vk::RenderPassBeginInfo {
                s_type: vk::StructureType::RENDER_PASS_BEGIN_INFO,
                p_next: ptr::null(),
                render_area,
                framebuffer: framebuffers[i],
                render_pass,
                clear_value_count: 1,
                p_clear_values: clear_values.as_ptr(),
                _marker: PhantomData,
            };
            unsafe {
                device.begin_command_buffer(command_buffer, &begin_info)?;
                device.cmd_begin_render_pass(
                    command_buffer,
                    &render_pass_begin,
                    vk::SubpassContents::INLINE,
                );
                device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline);
                device.cmd_draw(command_buffer, 3, 1, 0, 0);
                device.cmd_end_render_pass(command_buffer);
                device.end_command_buffer(command_buffer)?;
            };
        }

        Ok(())
    }

    fn create_semaphores_and_fences(
        images_len: usize,
        device: &ash::Device,
    ) -> VkResult<(Vec<vk::Semaphore>, Vec<vk::Semaphore>, Vec<vk::Fence>)> {
        let (mut available, mut finished, mut can_draw) = (
            Vec::with_capacity(images_len),
            Vec::with_capacity(images_len),
            Vec::with_capacity(images_len),
        );
        let semaphore_create_info = vk::SemaphoreCreateInfo {
            s_type: vk::StructureType::SEMAPHORE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::SemaphoreCreateFlags::empty(),
            _marker: PhantomData,
        };
        let fence_create_info = vk::FenceCreateInfo {
            s_type: vk::StructureType::FENCE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::FenceCreateFlags::SIGNALED,
            _marker: PhantomData,
        };

        for _ in 0..images_len {
            unsafe {
                available.push(device.create_semaphore(&semaphore_create_info, None)?);
                finished.push(device.create_semaphore(&semaphore_create_info, None)?);
                can_draw.push(device.create_fence(&fence_create_info, None)?);
            };
        }

        Ok((available, finished, can_draw))
    }

    #[inline]
    fn draw(&mut self) -> VkResult<()> {
        let current_img = (self.current_image + 1) % self.images.len();
        self.current_image = current_img;

        unsafe {
            let (img_index, _) = self.swapchain_loader.acquire_next_image(
                self.swapchain,
                u64::MAX,
                self.image_available[current_img],
                vk::Fence::null(),
            )?;

            self.device
                .wait_for_fences(&[self.can_draw[current_img]], true, u64::MAX)?;

            self.device.reset_fences(&[self.can_draw[current_img]])?;

            let submit_info = [vk::SubmitInfo {
                s_type: vk::StructureType::SUBMIT_INFO,
                p_next: ptr::null(),
                wait_semaphore_count: 1,
                p_wait_semaphores: [self.image_available[current_img]].as_ptr(),
                p_wait_dst_stage_mask: [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT].as_ptr(),
                command_buffer_count: 1,
                p_command_buffers: [self.command_buffers[current_img]].as_ptr(),
                signal_semaphore_count: 1,
                p_signal_semaphores: [self.rendering_finished[current_img]].as_ptr(),
                _marker: PhantomData,
            }];

            self.device.queue_submit(
                self.present_graphics_queue,
                &submit_info,
                self.can_draw[current_img],
            )?;

            let present_info = vk::PresentInfoKHR {
                s_type: vk::StructureType::PRESENT_INFO_KHR,
                p_next: ptr::null(),
                wait_semaphore_count: 1,
                p_wait_semaphores: [self.rendering_finished[current_img]].as_ptr(),
                swapchain_count: 1,
                p_swapchains: [self.swapchain].as_ptr(),
                p_image_indices: [img_index].as_ptr(),
                p_results: ptr::null_mut(),
                _marker: PhantomData,
            };

            self.swapchain_loader
                .queue_present(self.present_graphics_queue, &present_info)?;
        };
        Ok(())
    }

    fn recreate_swapchain(&mut self) -> VkResult<()> {
        unsafe { self.device.device_wait_idle()? };
        Ok(())
    }
}

impl winit::application::ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.renderer.is_none() {
            self.renderer =
                Some(Renderer::new(event_loop).unwrap_or_else(|err| {
                    panic!("Error occured while creating Renderer : {}", err)
                }))
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
            winit::event::WindowEvent::CloseRequested => {
                unsafe {
                    self.renderer
                        .as_ref()
                        .unwrap()
                        .device
                        .device_wait_idle()
                        .unwrap();
                }
                event_loop.exit();
            }
            winit::event::WindowEvent::RedrawRequested => {
                self.renderer
                    .as_mut()
                    .unwrap()
                    .draw()
                    .unwrap_or_else(|err| println!("Error while drawing : {}", err));
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        self.renderer.as_ref().unwrap().window.request_redraw();
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            for i in 0..self.can_draw.len() {
                self.device.destroy_semaphore(self.image_available[i], None);
                self.device
                    .destroy_semaphore(self.rendering_finished[i], None);
                self.device.destroy_fence(self.can_draw[i], None);
            }
            self.device.destroy_command_pool(self.command_pool, None);
            self.device.destroy_pipeline(self.pipeline, None);
            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None);
            for i in 0..self.framebuffers.len() {
                self.device.destroy_framebuffer(self.framebuffers[i], None);
            }
            self.device.destroy_render_pass(self.render_pass, None);
            for i in 0..self.image_views.len() {
                self.device.destroy_image_view(self.image_views[i], None);
            }
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.debug_utils
                .destroy_debug_utils_messenger(self.debug_utils_messenger, None);
            self.instance.destroy_instance(None);
        };
    }
}
