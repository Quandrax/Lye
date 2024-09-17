use ash::{
    prelude::VkResult,
    vk::{self},
};
use core::panic;
use std::{
    ffi::{CStr, CString},
    ptr,
};
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

pub struct VulkanRenderer {
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
    format: vk::Format,
    extent: vk::Extent2D,
    images: Vec<vk::Image>,
    image_views: Vec<vk::ImageView>,
    renderpass: vk::RenderPass,
    framebuffers: Vec<vk::Framebuffer>,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    image_available: Vec<vk::Semaphore>,
    render_finished: Vec<vk::Semaphore>,
    can_draw: Vec<vk::Fence>,
    current_image: usize,
}

impl VulkanRenderer {
    pub fn new(event_loop: &ActiveEventLoop) -> Result<Self, vk::Result> {
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
        let (swapchain, swapchain_loader, format, extent, images, image_views) =
            VulkanRenderer::create_swapchain(
                &instance,
                &surface_loader,
                physical_device,
                gq_family_index,
                surface,
                &device,
            )?;
        let renderpass = VulkanRenderer::create_render_pass(&device, format)?;
        let framebuffers =
            VulkanRenderer::create_framebuffers(&device, renderpass, &image_views, extent)?;
        let (command_pool, command_buffers) =
            VulkanRenderer::create_command_buffers(gq_family_index, &device, images.len())?;
        let (pipeline, pipeline_layout) =
            VulkanRenderer::create_pipeline(&device, renderpass, extent)?;

        let (image_available, render_finished, can_draw) =
            VulkanRenderer::create_semaphores_and_fences(images.len(), &device)?;

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
            format,
            extent,
            images,
            image_views,
            renderpass,
            framebuffers,
            pipeline,
            pipeline_layout,
            command_pool,
            command_buffers,
            image_available,
            render_finished,
            can_draw,
            current_image: 0,
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
    ) -> VkResult<(
        vk::SwapchainKHR,
        ash::khr::swapchain::Device,
        vk::Format,
        vk::Extent2D,
        Vec<vk::Image>,
        Vec<vk::ImageView>,
    )> {
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
                    y
                }
            }
        };

        unsafe {
            println!(
                "Min image count {}, format : {:?}, resolution : {:?}",
                surface_loader
                    .get_physical_device_surface_capabilities(physical_device, surface)
                    .unwrap()
                    .min_image_count,
                image_format,
                img_resolution
            )
        };

        let swapchain_loader = ash::khr::swapchain::Device::new(instance, device);
        let swapchain_create_info = vk::SwapchainCreateInfoKHR {
            s_type: vk::StructureType::SWAPCHAIN_CREATE_INFO_KHR,
            p_next: ptr::null(),
            flags: vk::SwapchainCreateFlagsKHR::empty(),
            surface,
            min_image_count: 3,
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

        let swapchain = unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None)? };
        let images = unsafe { swapchain_loader.get_swapchain_images(swapchain)? };
        let mut image_views = Vec::with_capacity(images.len());

        for image in images.iter() {
            let img_create_info = vk::ImageViewCreateInfo {
                s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
                p_next: ptr::null(),
                flags: vk::ImageViewCreateFlags::empty(),
                image: *image,
                view_type: vk::ImageViewType::TYPE_2D,
                format: image_format.format,
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
                _marker: Default::default(),
            };

            image_views.push(unsafe { device.create_image_view(&img_create_info, None)? });
        }

        Ok((
            swapchain,
            swapchain_loader,
            image_format.format,
            img_resolution,
            images,
            image_views,
        ))
    }

    fn create_render_pass(device: &ash::Device, format: vk::Format) -> VkResult<vk::RenderPass> {
        let attachment = vk::AttachmentDescription {
            flags: vk::AttachmentDescriptionFlags::empty(),
            format,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
            stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
            stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
        };

        let color_attachment_ref = vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        };

        let subpass = vk::SubpassDescription {
            flags: vk::SubpassDescriptionFlags::empty(),
            p_color_attachments: &color_attachment_ref,
            pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
            ..Default::default()
        };

        let subpass_dependencies = vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            dst_subpass: 0,
            src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            src_access_mask: vk::AccessFlags::empty(),
            dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_READ
                | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            dependency_flags: vk::DependencyFlags::empty(),
        };

        let renderpass_create_info = vk::RenderPassCreateInfo {
            s_type: vk::StructureType::RENDER_PASS_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::RenderPassCreateFlags::empty(),
            attachment_count: 1,
            p_attachments: &attachment,
            subpass_count: 1,
            p_subpasses: &subpass,
            dependency_count: 1,
            p_dependencies: &subpass_dependencies,
            _marker: Default::default(),
        };

        Ok(unsafe { device.create_render_pass(&renderpass_create_info, None)? })
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

    fn create_pipeline(
        device: &ash::Device,
        render_pass: vk::RenderPass,
        extent: vk::Extent2D,
    ) -> VkResult<(vk::Pipeline, vk::PipelineLayout)> {
        let mut cursor = std::io::Cursor::new(&include_bytes!("../shaders/vertex.spv")[..]);
        let vertex_bytes = ash::util::read_spv(&mut cursor).unwrap();
        let vertex_create_info = vk::ShaderModuleCreateInfo::default().code(&vertex_bytes);
        let vertex_module = unsafe { device.create_shader_module(&vertex_create_info, None)? };

        cursor = std::io::Cursor::new(&include_bytes!("../shaders/fragment.spv")[..]);
        let fragment_bytes = ash::util::read_spv(&mut cursor).unwrap();
        let fragment_create_info = vk::ShaderModuleCreateInfo::default().code(&fragment_bytes);
        let fragment_module = unsafe { device.create_shader_module(&fragment_create_info, None)? };

        let entry = CString::new("main").unwrap();

        let shader_states_create_infos = [
            vk::PipelineShaderStageCreateInfo {
                s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
                p_next: ptr::null(),
                flags: vk::PipelineShaderStageCreateFlags::empty(),
                stage: vk::ShaderStageFlags::VERTEX,
                module: vertex_module,
                p_name: entry.as_ptr(),
                p_specialization_info: ptr::null(),
                _marker: Default::default(),
            },
            vk::PipelineShaderStageCreateInfo {
                s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
                p_next: ptr::null(),
                flags: vk::PipelineShaderStageCreateFlags::empty(),
                stage: vk::ShaderStageFlags::FRAGMENT,
                module: fragment_module,
                p_name: entry.as_ptr(),
                p_specialization_info: ptr::null(),
                _marker: Default::default(),
            },
        ];

        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineVertexInputStateCreateFlags::empty(),
            vertex_attribute_description_count: 0,
            vertex_binding_description_count: 0,
            p_vertex_attribute_descriptions: ptr::null(),
            p_vertex_binding_descriptions: ptr::null(),
            _marker: Default::default(),
        };

        let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineInputAssemblyStateCreateFlags::empty(),
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            primitive_restart_enable: 0,
            _marker: Default::default(),
        };

        let viewports = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: extent.width as f32,
            height: extent.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];

        let scissors = [vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent,
        }];

        let viewport_state = vk::PipelineViewportStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_VIEWPORT_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineViewportStateCreateFlags::empty(),
            viewport_count: 1,
            p_viewports: viewports.as_ptr(),
            scissor_count: 1,
            p_scissors: scissors.as_ptr(),
            _marker: Default::default(),
        };

        let rasterization_state = vk::PipelineRasterizationStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_RASTERIZATION_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineRasterizationStateCreateFlags::empty(),
            depth_clamp_enable: 0,
            rasterizer_discard_enable: 0,
            polygon_mode: vk::PolygonMode::FILL,
            cull_mode: vk::CullModeFlags::BACK,
            front_face: vk::FrontFace::COUNTER_CLOCKWISE,
            depth_bias_enable: 0,
            depth_bias_constant_factor: 0.0,
            depth_bias_clamp: 0.0,
            depth_bias_slope_factor: 0.0,
            line_width: 1.0,
            _marker: Default::default(),
        };

        let multisample_state = vk::PipelineMultisampleStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_MULTISAMPLE_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineMultisampleStateCreateFlags::empty(),
            rasterization_samples: vk::SampleCountFlags::TYPE_1,
            sample_shading_enable: 0,
            min_sample_shading: 1.0,
            p_sample_mask: ptr::null(),
            alpha_to_coverage_enable: 0,
            alpha_to_one_enable: 0,
            _marker: Default::default(),
        };

        let color_blend_attachments = [vk::PipelineColorBlendAttachmentState {
            blend_enable: 0,
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
            logic_op_enable: 0,
            attachment_count: 1,
            p_attachments: color_blend_attachments.as_ptr(),
            blend_constants: [0.0, 0.0, 0.0, 0.0],
            _marker: Default::default(),
        };

        let layout_info = vk::PipelineLayoutCreateInfo::default();
        let pipeline_layout = unsafe { device.create_pipeline_layout(&layout_info, None)? };

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
            base_pipeline_handle: vk::Pipeline::default(),
            base_pipeline_index: Default::default(),
            _marker: Default::default(),
        }];

        let pipeline = unsafe {
            device
                .create_graphics_pipelines(vk::PipelineCache::null(), &pipeline_create_info, None)
                .unwrap()[0]
        };

        unsafe {
            device.destroy_shader_module(vertex_module, None);
            device.destroy_shader_module(fragment_module, None);
        };

        Ok((pipeline, pipeline_layout))
    }

    fn create_command_buffers(
        queue_family_index: u32,
        device: &ash::Device,
        count: usize,
    ) -> VkResult<(vk::CommandPool, Vec<vk::CommandBuffer>)> {
        let commandpool_create_info = vk::CommandPoolCreateInfo {
            s_type: vk::StructureType::COMMAND_POOL_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            queue_family_index,
            _marker: Default::default(),
        };

        let command_pool = unsafe { device.create_command_pool(&commandpool_create_info, None)? };

        let allocate_info = vk::CommandBufferAllocateInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
            p_next: ptr::null(),
            command_pool,
            level: vk::CommandBufferLevel::PRIMARY,
            command_buffer_count: count as u32,
            _marker: Default::default(),
        };

        let command_buffers = unsafe { device.allocate_command_buffers(&allocate_info)? };

        Ok((command_pool, command_buffers))
    }

    fn record_command_buffer(
        device: &ash::Device,
        command_buffers: &[vk::CommandBuffer],
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
            _marker: Default::default(),
        };
        let clear_values = [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        }];

        for (i, &command_buffer) in command_buffers.iter().enumerate() {
            unsafe { device.begin_command_buffer(command_buffer, &begin_info)? };
            let render_pass_begin_info = vk::RenderPassBeginInfo {
                s_type: vk::StructureType::RENDER_PASS_BEGIN_INFO,
                p_next: ptr::null(),
                render_pass,
                framebuffer: framebuffers[i],
                render_area: vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent,
                },
                clear_value_count: 1,
                p_clear_values: clear_values.as_ptr(),
                _marker: Default::default(),
            };
            unsafe {
                device.cmd_begin_render_pass(
                    command_buffer,
                    &render_pass_begin_info,
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
            _marker: Default::default(),
        };
        let fence_create_info = vk::FenceCreateInfo {
            s_type: vk::StructureType::FENCE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::FenceCreateFlags::SIGNALED,
            _marker: Default::default(),
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

    fn current_image(&mut self) {
        self.current_image = (self.current_image + 1) % self.image_views.len();
    }
}

impl ApplicationHandler for IHateWinitVer30 {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.game.is_none() {
            self.game = Some(VulkanRenderer::new(event_loop).unwrap_or_else(|err| {
                println!("Error : {}", err);
                panic!()
            }));
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
            WindowEvent::RedrawRequested => {
                let renderer = self.game.as_mut().unwrap();
            }
            _ => (),
        }
    }
}

impl Drop for VulkanRenderer {
    fn drop(&mut self) {
        unsafe {
            for semaphore in self.image_available.iter() {
                self.device.destroy_semaphore(*semaphore, None);
            }
            for semaphore in self.render_finished.iter() {
                self.device.destroy_semaphore(*semaphore, None);
            }
            for fence in self.can_draw.iter() {
                self.device.destroy_fence(*fence, None);
            }
            self.device.destroy_command_pool(self.command_pool, None);
            self.device.destroy_pipeline(self.pipeline, None);
            for framebuffer in self.framebuffers.iter_mut() {
                self.device.destroy_framebuffer(*framebuffer, None);
            }
            self.device.destroy_render_pass(self.renderpass, None);
            for image_view in self.image_views.iter_mut() {
                self.device.destroy_image_view(*image_view, None)
            }
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.instance.destroy_instance(None);
        };
    }
}
