mod image;
mod instance;
mod physical_surface;

pub use instance::*;
use std::ops::Drop;
use ash::vk;
use std::ffi;
use std::borrow::Cow;
use ash::ext::debug_utils as ash_debug_utils;
use ash::khr::{surface, swapchain as ash_swapchain};
use ash::vk::{DebugUtilsMessengerEXT, PhysicalDevice};
use winit::raw_window_handle::{HasDisplayHandle, HasRawDisplayHandle, HasWindowHandle, RawDisplayHandle};
use std::ffi::c_char;
use std::io::Cursor;
use ash::util::read_spv;
use ash_window;
use winit::window::Window;
use crate::Vertex;
use crate::vulkan::physical_surface::PhysicalSurface;

pub fn find_memorytype_index(
    memory_req: &vk::MemoryRequirements,
    memory_prop: &vk::PhysicalDeviceMemoryProperties,
    flags: vk::MemoryPropertyFlags,
) -> Option<u32> {
    memory_prop.memory_types[..memory_prop.memory_type_count as _]
        .iter()
        .enumerate()
        .find(|(index, memory_type)| {
            (1u32 << index) & memory_req.memory_type_bits != 0
                && memory_type.property_flags & flags == flags
        })
        .map(|(index, _memory_type)| index as _)
}

/// Helper function for submitting command buffers. Immediately waits for the fence before the command buffer
/// is executed. That way we can delay the waiting for the fences by 1 frame which is good for performance.
/// Make sure to create the fence in a signaled state on the first use.
#[allow(clippy::too_many_arguments)]
pub fn record_submit_commandbuffer<F: FnOnce(&ash::Device, vk::CommandBuffer)>(
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    command_buffer_reuse_fence: vk::Fence,
    submit_queue: vk::Queue,
    wait_mask: &[vk::PipelineStageFlags],
    wait_semaphores: &[vk::Semaphore],
    signal_semaphores: &[vk::Semaphore],
    f: F,
) {
    unsafe {
        device
            .wait_for_fences(&[command_buffer_reuse_fence], true, u64::MAX)
            .expect("Wait for fence failed.");

        device
            .reset_fences(&[command_buffer_reuse_fence])
            .expect("Reset fences failed.");

        device
            .reset_command_buffer(
                command_buffer,
                vk::CommandBufferResetFlags::RELEASE_RESOURCES,
            )
            .expect("Reset command buffer failed.");

        let command_buffer_begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        device
            .begin_command_buffer(command_buffer, &command_buffer_begin_info)
            .expect("Begin commandbuffer");
        f(device, command_buffer);
        device
            .end_command_buffer(command_buffer)
            .expect("End commandbuffer");

        let command_buffers = vec![command_buffer];

        let submit_info = vk::SubmitInfo::default()
            .wait_semaphores(wait_semaphores)
            .wait_dst_stage_mask(wait_mask)
            .command_buffers(&command_buffers)
            .signal_semaphores(signal_semaphores);

        device
            .queue_submit(submit_queue, &[submit_info], command_buffer_reuse_fence)
            .expect("queue submit failed.");
    }
}
/// Vulkan Context which contains physical device, logical device, and surface, etc.
/// There will probably be a pointer of this being passed around

pub struct VulkanContext {
    pub instance: Instance,
    pub physical_surface: PhysicalSurface,
    pub device : ash::Device,
    pub swapchain_loader: ash::khr::swapchain::Device,
    pub device_memory_properties: vk::PhysicalDeviceMemoryProperties,
    pub present_queue: vk::Queue,
    pub surface_format: vk::SurfaceFormatKHR,
    pub surface_resolution: vk::Extent2D,

    pub swapchain: vk::SwapchainKHR,
    pub new_swapchain_size: Option<vk::Extent2D>,

    pub present_images: Vec<vk::Image>,
    pub present_image_views: Vec<vk::ImageView>,

    pub pool: vk::CommandPool,
    pub draw_command_buffers: Vec<vk::CommandBuffer>,
    pub setup_command_buffer: vk::CommandBuffer,
    pub setup_commands_reuse_fence : vk::Fence,

    pub depth_images: Vec<vk::Image>,
    pub depth_image_views: Vec<vk::ImageView>,
    pub depth_image_memories: Vec<vk::DeviceMemory>,

    pub image_available_semaphores: Vec<vk::Semaphore>,
    pub rendering_complete_semaphores: Vec<vk::Semaphore>,
    pub frames_in_flight_fences: Vec<vk::Fence>,

    pub render_pass: vk::RenderPass,
    pub framebuffers: Vec<vk::Framebuffer>,
    pub graphics_pipeline: vk::Pipeline,
    pub vertex_input_buffer: vk::Buffer,
    pub vertex_input_buffer_memory: vk::DeviceMemory,
    pub index_buffer: vk::Buffer,
    pub index_buffer_memory: vk::DeviceMemory,

    pub vertex_shader_module: vk::ShaderModule,
    pub fragment_shader_module: vk::ShaderModule,
    pub pipeline_layout: vk::PipelineLayout,
}

impl VulkanContext {
    pub unsafe fn new(window: &Box<dyn Window>) -> Self {

        let instance = Instance::new(window);
        let physical_surface = PhysicalSurface::new(&instance, window);

        let device_extension_names_raw = [
            ash_swapchain::NAME.as_ptr(),
        ];

        let features = vk::PhysicalDeviceFeatures {
            shader_clip_distance: 1,
            ..Default::default()
        };
        let priorities = [1.0];

        let queue_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(physical_surface.queue_family_index)
            .queue_priorities(&priorities);

        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(std::slice::from_ref(&queue_info))
            .enabled_extension_names(&device_extension_names_raw)
            .enabled_features(&features);

        let device: ash::Device = instance.instance
            .create_device(physical_surface.physical_device, &device_create_info, None)
            .unwrap();


        let present_queue = device.get_device_queue(physical_surface.queue_family_index, 0);

        let surface_format = physical_surface.surface_loader
            .get_physical_device_surface_formats(physical_surface.physical_device, physical_surface.surface)
            .unwrap()[0];

        let surface_capabilities = physical_surface.surface_loader
            .get_physical_device_surface_capabilities(physical_surface.physical_device, physical_surface.surface)
            .unwrap();

        let mut desired_image_count = surface_capabilities.min_image_count + 1;
        if surface_capabilities.max_image_count > 0
            && desired_image_count > surface_capabilities.max_image_count
        {
            desired_image_count = surface_capabilities.max_image_count;
        }
        let window_size = window.surface_size();
        let surface_resolution = match surface_capabilities.current_extent.width {
            u32::MAX => vk::Extent2D {
                width: window_size.width,
                height: window_size.height,
            },
            _ => surface_capabilities.current_extent,
        };

        let pre_transform = if surface_capabilities
            .supported_transforms
            .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
        {
            vk::SurfaceTransformFlagsKHR::IDENTITY
        } else {
            surface_capabilities.current_transform
        };
        let present_modes = physical_surface.surface_loader
            .get_physical_device_surface_present_modes(physical_surface.physical_device, physical_surface.surface)
            .unwrap();
        let present_mode = present_modes
            .iter()
            .cloned()
            .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
            .unwrap_or(vk::PresentModeKHR::FIFO);
        let swapchain_loader = ash_swapchain::Device::new(&instance.instance, &device);

        let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(physical_surface.surface)
            .min_image_count(desired_image_count)
            .image_color_space(surface_format.color_space)
            .image_format(surface_format.format)
            .image_extent(surface_resolution)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(pre_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .image_array_layers(1);

        let swapchain = swapchain_loader
            .create_swapchain(&swapchain_create_info, None)
            .unwrap();

        let present_images = swapchain_loader.get_swapchain_images(swapchain).unwrap();
        let present_image_views: Vec<vk::ImageView> = present_images
            .iter()
            .map(|&image| {
                let create_view_info = vk::ImageViewCreateInfo::default()
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(surface_format.format)
                    .components(vk::ComponentMapping {
                        r: vk::ComponentSwizzle::R,
                        g: vk::ComponentSwizzle::G,
                        b: vk::ComponentSwizzle::B,
                        a: vk::ComponentSwizzle::A,
                    })
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .image(image);
                device.create_image_view(&create_view_info, None).unwrap()
            })
            .collect();

        let n_frame_buffers = present_images.len();

        let pool_create_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(physical_surface.queue_family_index);

        let pool = device.create_command_pool(&pool_create_info, None).unwrap();

        let setup_command_buffer_allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_buffer_count((1) as u32)
            .command_pool(pool)
            .level(vk::CommandBufferLevel::PRIMARY);

        let setup_command_buffers = device
            .allocate_command_buffers(&setup_command_buffer_allocate_info)
            .unwrap();

        let setup_command_buffer = setup_command_buffers[0];

        let draw_command_buffer_allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_buffer_count((n_frame_buffers) as u32)
            .command_pool(pool)
            .level(vk::CommandBufferLevel::PRIMARY);

        let draw_command_buffers = device.allocate_command_buffers(&draw_command_buffer_allocate_info).unwrap();

        let device_memory_properties = instance.instance.get_physical_device_memory_properties(physical_surface.physical_device);

        let mut depth_images = Vec::new();
        let mut depth_image_views = Vec::new();
        let mut depth_image_memories = Vec::new();

        let fence_create_info =
            vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);

        let setup_commands_reuse_fence = device
            .create_fence(&fence_create_info, None)
            .expect("Create fence failed.");

        for _ in 0..n_frame_buffers {
            let depth_image_create_info = vk::ImageCreateInfo::default()
                .image_type(vk::ImageType::TYPE_2D)
                .format(vk::Format::D16_UNORM)
                .extent(surface_resolution.into())
                .mip_levels(1)
                .array_layers(1)
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::OPTIMAL)
                .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);
            let depth_image = device.create_image(&depth_image_create_info, None).unwrap();
            let depth_image_memory_req = device.get_image_memory_requirements(depth_image);
            let depth_image_memory_index = find_memorytype_index(
                &depth_image_memory_req,
                &device_memory_properties,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            )
                .expect("Unable to find suitable memory index for depth image.");

            let depth_image_allocate_info = vk::MemoryAllocateInfo::default()
                .allocation_size(depth_image_memory_req.size)
                .memory_type_index(depth_image_memory_index);

            let depth_image_memory = device
                .allocate_memory(&depth_image_allocate_info, None)
                .unwrap();

            device
                .bind_image_memory(depth_image, depth_image_memory, 0)
                .expect("Unable to bind depth image memory");


            record_submit_commandbuffer(
                &device,
                setup_command_buffer,
                setup_commands_reuse_fence,
                present_queue,
                &[],
                &[],
                &[],
                |device, setup_command_buffer| {
                    let layout_transition_barriers = vk::ImageMemoryBarrier::default()
                        .image(depth_image)
                        .dst_access_mask(
                            vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                                | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
                        )
                        .new_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                        .old_layout(vk::ImageLayout::UNDEFINED)
                        .subresource_range(
                            vk::ImageSubresourceRange::default()
                                .aspect_mask(vk::ImageAspectFlags::DEPTH)
                                .layer_count(1)
                                .level_count(1),
                        );

                    device.cmd_pipeline_barrier(
                        setup_command_buffer,
                        vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                        vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        &[layout_transition_barriers],
                    );
                },
            );



            let depth_image_view_info = vk::ImageViewCreateInfo::default()
                .subresource_range(
                    vk::ImageSubresourceRange::default()
                        .aspect_mask(vk::ImageAspectFlags::DEPTH)
                        .level_count(1)
                        .layer_count(1),
                )
                .image(depth_image)
                .format(depth_image_create_info.format)
                .view_type(vk::ImageViewType::TYPE_2D);

            let depth_image_view = device
                .create_image_view(&depth_image_view_info, None)
                .unwrap();

            depth_images.push(depth_image);
            depth_image_views.push(depth_image_view);
            depth_image_memories.push(depth_image_memory);
        }

        let semaphore_create_info = vk::SemaphoreCreateInfo::default();

        let mut frames_in_flight_fences = Vec::new();
        let mut image_available_semaphores = Vec::new();
        let mut rendering_complete_semaphores = Vec::new();

        for _ in 0..n_frame_buffers {
            let fence = device.create_fence(&fence_create_info, None).unwrap();
            frames_in_flight_fences.push(fence);
            let present_complete_semaphore = device.create_semaphore(&semaphore_create_info, None).unwrap();
            let rendering_complete_semaphore = device.create_semaphore(&semaphore_create_info, None).unwrap();
            image_available_semaphores.push(present_complete_semaphore);
            rendering_complete_semaphores.push(rendering_complete_semaphore);
        }

        let renderpass_attachments = [
            vk::AttachmentDescription {
                format: surface_format.format,
                samples: vk::SampleCountFlags::TYPE_1,
                load_op: vk::AttachmentLoadOp::CLEAR,
                store_op: vk::AttachmentStoreOp::STORE,
                final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
                ..Default::default()
            },
            vk::AttachmentDescription {
                format: vk::Format::D16_UNORM,
                samples: vk::SampleCountFlags::TYPE_1,
                load_op: vk::AttachmentLoadOp::CLEAR,
                initial_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                final_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                ..Default::default()
            },
        ];

        let color_attachment_refs = [vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        }];
        let depth_attachment_ref = vk::AttachmentReference {
            attachment: 1,
            layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        };
        let dependencies = [vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_READ
                | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            ..Default::default()
        }];

        let subpass = vk::SubpassDescription::default()
            .color_attachments(&color_attachment_refs)
            .depth_stencil_attachment(&depth_attachment_ref)
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS);

        let renderpass_create_info = vk::RenderPassCreateInfo::default()
            .attachments(&renderpass_attachments)
            .subpasses(std::slice::from_ref(&subpass))
            .dependencies(&dependencies);

        let render_pass = device
            .create_render_pass(&renderpass_create_info, None)
            .unwrap();

        let framebuffers: Vec<vk::Framebuffer> = present_image_views.iter().enumerate()
            .map(|(index, &present_image_view)| {
                let framebuffer_attachments = [present_image_view, depth_image_views[index]];
                let frame_buffer_create_info = vk::FramebufferCreateInfo::default()
                    .render_pass(render_pass)
                    .attachments(&framebuffer_attachments)
                    .width(surface_resolution.width)
                    .height(surface_resolution.height)
                    .layers(1);

                device
                    .create_framebuffer(&frame_buffer_create_info, None)
                    .unwrap()
            })
            .collect();

        let index_buffer_data = [0u32, 1, 2];
        let index_buffer_info = vk::BufferCreateInfo::default()
            .size(size_of_val(&index_buffer_data) as u64)
            .usage(vk::BufferUsageFlags::INDEX_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let index_buffer = device.create_buffer(&index_buffer_info, None).unwrap();
        let index_buffer_memory_req = device.get_buffer_memory_requirements(index_buffer);
        let index_buffer_memory_index = find_memorytype_index(
            &index_buffer_memory_req,
            &device_memory_properties,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )
            .expect("Unable to find suitable memorytype for the index buffer.");

        let index_allocate_info = vk::MemoryAllocateInfo {
            allocation_size: index_buffer_memory_req.size,
            memory_type_index: index_buffer_memory_index,
            ..Default::default()
        };
        let index_buffer_memory = device
            .allocate_memory(&index_allocate_info, None)
            .unwrap();
        let index_ptr = device
            .map_memory(
                index_buffer_memory,
                0,
                index_buffer_memory_req.size,
                vk::MemoryMapFlags::empty(),
            )
            .unwrap();
        let mut index_slice = ash::util::Align::new(
            index_ptr,
            align_of::<u32>() as u64,
            index_buffer_memory_req.size,
        );
        index_slice.copy_from_slice(&index_buffer_data);
        device.unmap_memory(index_buffer_memory);
        device
            .bind_buffer_memory(index_buffer, index_buffer_memory, 0)
            .unwrap();

        let vertex_input_buffer_info = vk::BufferCreateInfo {
            size: 3 * size_of::<Vertex>() as u64,
            usage: vk::BufferUsageFlags::VERTEX_BUFFER,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let vertex_input_buffer = device
            .create_buffer(&vertex_input_buffer_info, None)
            .unwrap();

        let vertex_input_buffer_memory_req = device
            .get_buffer_memory_requirements(vertex_input_buffer);

        let vertex_input_buffer_memory_index = find_memorytype_index(
            &vertex_input_buffer_memory_req,
            &device_memory_properties,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )
            .expect("Unable to find suitable memorytype for the vertex buffer.");

        let vertex_buffer_allocate_info = vk::MemoryAllocateInfo {
            allocation_size: vertex_input_buffer_memory_req.size,
            memory_type_index: vertex_input_buffer_memory_index,
            ..Default::default()
        };

        let vertex_input_buffer_memory = device
            .allocate_memory(&vertex_buffer_allocate_info, None)
            .unwrap();

        let vertices = [
            Vertex {
                pos: [-1.0, 1.0, 0.0, 1.0],
                color: [0.0, 1.0, 0.0, 1.0],
            },
            Vertex {
                pos: [1.0, 1.0, 0.0, 1.0],
                color: [0.0, 0.0, 1.0, 1.0],
            },
            Vertex {
                pos: [0.0, -1.0, 0.0, 1.0],
                color: [1.0, 0.0, 0.0, 1.0],
            },
        ];

        let vert_ptr = device
            .map_memory(
                vertex_input_buffer_memory,
                0,
                vertex_input_buffer_memory_req.size,
                vk::MemoryMapFlags::empty(),
            )
            .unwrap();

        let mut vert_align = ash::util::Align::new(
            vert_ptr,
            align_of::<Vertex>() as u64,
            vertex_input_buffer_memory_req.size,
        );
        vert_align.copy_from_slice(&vertices);
        device.unmap_memory(vertex_input_buffer_memory);
        device
            .bind_buffer_memory(vertex_input_buffer, vertex_input_buffer_memory, 0)
            .unwrap();

        let mut vertex_spv_file =
            Cursor::new(&include_bytes!("../../shaders/vert.spv")[..]);
        let mut frag_spv_file = Cursor::new(&include_bytes!("../../shaders/frag.spv")[..]);

        let vertex_code =
            read_spv(&mut vertex_spv_file).expect("Failed to read vertex shader spv file");
        let vertex_shader_info = vk::ShaderModuleCreateInfo::default().code(&vertex_code);

        let frag_code =
            read_spv(&mut frag_spv_file).expect("Failed to read fragment shader spv file");
        let frag_shader_info = vk::ShaderModuleCreateInfo::default().code(&frag_code);

        let vertex_shader_module = device
            .create_shader_module(&vertex_shader_info, None)
            .expect("Vertex shader module error");

        let fragment_shader_module = device
            .create_shader_module(&frag_shader_info, None)
            .expect("Fragment shader module error");

        let layout_create_info = vk::PipelineLayoutCreateInfo::default();

        let pipeline_layout = device
            .create_pipeline_layout(&layout_create_info, None)
            .unwrap();

        let shader_entry_name = c"main";
        let shader_stage_create_infos = [
            vk::PipelineShaderStageCreateInfo {
                module: vertex_shader_module,
                p_name: shader_entry_name.as_ptr(),
                stage: vk::ShaderStageFlags::VERTEX,
                ..Default::default()
            },
            vk::PipelineShaderStageCreateInfo {
                s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
                module: fragment_shader_module,
                p_name: shader_entry_name.as_ptr(),
                stage: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
        ];
        let vertex_input_binding_descriptions = [vk::VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<Vertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }];
        let vertex_input_attribute_descriptions = [
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: std::mem::offset_of!(Vertex, pos) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: std::mem::offset_of!(Vertex, color) as u32,
            },
        ];

        let vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_attribute_descriptions(&vertex_input_attribute_descriptions)
            .vertex_binding_descriptions(&vertex_input_binding_descriptions);
        let vertex_input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo {
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            ..Default::default()
        };
        let viewports = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: surface_resolution.width as f32,
            height: surface_resolution.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];
        let scissors = [surface_resolution.into()];
        let viewport_state_info = vk::PipelineViewportStateCreateInfo::default()
            .scissors(&scissors)
            .viewports(&viewports);

        let rasterization_info = vk::PipelineRasterizationStateCreateInfo {
            front_face: vk::FrontFace::COUNTER_CLOCKWISE,
            line_width: 1.0,
            polygon_mode: vk::PolygonMode::FILL,
            ..Default::default()
        };
        let multisample_state_info = vk::PipelineMultisampleStateCreateInfo {
            rasterization_samples: vk::SampleCountFlags::TYPE_1,
            ..Default::default()
        };
        let noop_stencil_state = vk::StencilOpState {
            fail_op: vk::StencilOp::KEEP,
            pass_op: vk::StencilOp::KEEP,
            depth_fail_op: vk::StencilOp::KEEP,
            compare_op: vk::CompareOp::ALWAYS,
            ..Default::default()
        };
        let depth_state_info = vk::PipelineDepthStencilStateCreateInfo {
            depth_test_enable: 1,
            depth_write_enable: 1,
            depth_compare_op: vk::CompareOp::LESS_OR_EQUAL,
            front: noop_stencil_state,
            back: noop_stencil_state,
            max_depth_bounds: 1.0,
            ..Default::default()
        };
        let color_blend_attachment_states = [vk::PipelineColorBlendAttachmentState {
            blend_enable: 0,
            src_color_blend_factor: vk::BlendFactor::SRC_COLOR,
            dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_DST_COLOR,
            color_blend_op: vk::BlendOp::ADD,
            src_alpha_blend_factor: vk::BlendFactor::ZERO,
            dst_alpha_blend_factor: vk::BlendFactor::ZERO,
            alpha_blend_op: vk::BlendOp::ADD,
            color_write_mask: vk::ColorComponentFlags::RGBA,
        }];
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op(vk::LogicOp::CLEAR)
            .attachments(&color_blend_attachment_states);

        let dynamic_state = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state_info =
            vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_state);

        let graphic_pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&shader_stage_create_infos)
            .vertex_input_state(&vertex_input_state_info)
            .input_assembly_state(&vertex_input_assembly_state_info)
            .viewport_state(&viewport_state_info)
            .rasterization_state(&rasterization_info)
            .multisample_state(&multisample_state_info)
            .depth_stencil_state(&depth_state_info)
            .color_blend_state(&color_blend_state)
            .dynamic_state(&dynamic_state_info)
            .layout(pipeline_layout)
            .render_pass(render_pass);

        let graphics_pipelines = device
            .create_graphics_pipelines(vk::PipelineCache::null(), &[graphic_pipeline_info], None)
            .expect("Unable to create graphics pipeline");

        let graphics_pipeline = graphics_pipelines[0];



        Self {
            instance,
            physical_surface,
            device,
            swapchain_loader,
            device_memory_properties,
            present_queue,
            surface_format,
            surface_resolution,

            swapchain,
            new_swapchain_size: None,

            present_images,
            present_image_views,

            pool,
            draw_command_buffers,
            setup_command_buffer,
            setup_commands_reuse_fence,

            depth_images,
            depth_image_views,
            depth_image_memories,

            image_available_semaphores,
            rendering_complete_semaphores,

            frames_in_flight_fences,

            render_pass,
            framebuffers,
            graphics_pipeline,
            vertex_input_buffer,
            vertex_input_buffer_memory,
            index_buffer,
            index_buffer_memory,
            fragment_shader_module,
            vertex_shader_module,
            pipeline_layout
        }
    }

    pub unsafe fn recreate_swapchain(self: &mut Self, surface_size: vk::Extent2D) {
        self.device.device_wait_idle().unwrap();

        for framebuffer in self.framebuffers.iter() {
            self.device.destroy_framebuffer(*framebuffer, None);
        }

        for i in 0..self.framebuffers.len() {
            self.device.free_memory(self.depth_image_memories[i], None);
            self.device.destroy_image_view(self.depth_image_views[i], None);
            self.device.destroy_image(self.depth_images[i], None);
        }


        for &image_view in self.present_image_views.iter() {
            self.device.destroy_image_view(image_view, None);
        }


        self.swapchain_loader.destroy_swapchain(self.swapchain, None);

        let surface_capabilities = self.physical_surface.surface_loader
            .get_physical_device_surface_capabilities(self.physical_surface.physical_device, self.physical_surface.surface)
            .unwrap();


        let mut desired_image_count = surface_capabilities.min_image_count + 1;
        if surface_capabilities.max_image_count > 0
            && desired_image_count > surface_capabilities.max_image_count
        {
            desired_image_count = surface_capabilities.max_image_count;
        }

        self.surface_resolution = match surface_capabilities.current_extent.width {
            u32::MAX => vk::Extent2D {
                width: surface_size.width,
                height: surface_size.height,
            },
            _ => surface_capabilities.current_extent,
        };

        let pre_transform = if surface_capabilities
            .supported_transforms
            .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
        {
            vk::SurfaceTransformFlagsKHR::IDENTITY
        } else {
            surface_capabilities.current_transform
        };

        let present_modes = self.physical_surface.surface_loader
            .get_physical_device_surface_present_modes(self.physical_surface.physical_device, self.physical_surface.surface)
            .unwrap();
        let present_mode = present_modes
            .iter()
            .cloned()
            .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
            .unwrap_or(vk::PresentModeKHR::FIFO);
        let swapchain_loader = ash_swapchain::Device::new(&self.instance.instance, &self.device);

        let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(self.physical_surface.surface)
            .min_image_count(desired_image_count)
            .image_color_space(self.surface_format.color_space)
            .image_format(self.surface_format.format)
            .image_extent(self.surface_resolution)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(pre_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .image_array_layers(1);

        self.swapchain = swapchain_loader
            .create_swapchain(&swapchain_create_info, None)
            .unwrap();

        self.present_images = swapchain_loader.get_swapchain_images(self.swapchain).unwrap();
        self.present_image_views = self.present_images
            .iter()
            .map(|&image| {
                let create_view_info = vk::ImageViewCreateInfo::default()
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(self.surface_format.format)
                    .components(vk::ComponentMapping {
                        r: vk::ComponentSwizzle::R,
                        g: vk::ComponentSwizzle::G,
                        b: vk::ComponentSwizzle::B,
                        a: vk::ComponentSwizzle::A,
                    })
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .image(image);
                self.device.create_image_view(&create_view_info, None).unwrap()
            })
            .collect();

        let n_frame_buffers = self.present_images.len();

        let device_memory_properties = self.instance.instance.get_physical_device_memory_properties(self.physical_surface.physical_device);

        self.depth_images = Vec::new();
        self.depth_image_views = Vec::new();
        self.depth_image_memories = Vec::new();

        for _ in 0..n_frame_buffers {
            let depth_image_create_info = vk::ImageCreateInfo::default()
                .image_type(vk::ImageType::TYPE_2D)
                .format(vk::Format::D16_UNORM)
                .extent(self.surface_resolution.into())
                .mip_levels(1)
                .array_layers(1)
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::OPTIMAL)
                .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);
            let depth_image = self.device.create_image(&depth_image_create_info, None).unwrap();
            let depth_image_memory_req = self.device.get_image_memory_requirements(depth_image);
            let depth_image_memory_index = find_memorytype_index(
                &depth_image_memory_req,
                &device_memory_properties,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            )
                .expect("Unable to find suitable memory index for depth image.");

            let depth_image_allocate_info = vk::MemoryAllocateInfo::default()
                .allocation_size(depth_image_memory_req.size)
                .memory_type_index(depth_image_memory_index);

            let depth_image_memory = self.device
                .allocate_memory(&depth_image_allocate_info, None)
                .unwrap();

            self.device
                .bind_image_memory(depth_image, depth_image_memory, 0)
                .expect("Unable to bind depth image memory");


            record_submit_commandbuffer(
                &self.device,
                self.setup_command_buffer,
                self.setup_commands_reuse_fence,
                self.present_queue,
                &[],
                &[],
                &[],
                |device, setup_command_buffer| {
                    let layout_transition_barriers = vk::ImageMemoryBarrier::default()
                        .image(depth_image)
                        .dst_access_mask(
                            vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                                | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
                        )
                        .new_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                        .old_layout(vk::ImageLayout::UNDEFINED)
                        .subresource_range(
                            vk::ImageSubresourceRange::default()
                                .aspect_mask(vk::ImageAspectFlags::DEPTH)
                                .layer_count(1)
                                .level_count(1),
                        );

                    device.cmd_pipeline_barrier(
                        setup_command_buffer,
                        vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                        vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        &[layout_transition_barriers],
                    );
                },
            );

            let depth_image_view_info = vk::ImageViewCreateInfo::default()
                .subresource_range(
                    vk::ImageSubresourceRange::default()
                        .aspect_mask(vk::ImageAspectFlags::DEPTH)
                        .level_count(1)
                        .layer_count(1),
                )
                .image(depth_image)
                .format(depth_image_create_info.format)
                .view_type(vk::ImageViewType::TYPE_2D);

            let depth_image_view = self.device
                .create_image_view(&depth_image_view_info, None)
                .unwrap();

            self.depth_images.push(depth_image);
            self.depth_image_views.push(depth_image_view);
            self.depth_image_memories.push(depth_image_memory);
        }

        self.framebuffers = self.present_image_views
            .iter().enumerate()
            .map(|(index, &present_image_view)| {
                let framebuffer_attachments = [present_image_view, self.depth_image_views[index]];
                let frame_buffer_create_info = vk::FramebufferCreateInfo::default()
                    .render_pass(self.render_pass)
                    .attachments(&framebuffer_attachments)
                    .width(self.surface_resolution.width)
                    .height(self.surface_resolution.height)
                    .layers(1);

                self.device
                    .create_framebuffer(&frame_buffer_create_info, None)
                    .unwrap()
            })
            .collect();

    }
}

impl Drop for VulkanContext {
    fn drop(&mut self) {
        println!("Dropping application.");
        unsafe {
            self.device.device_wait_idle().unwrap();

            self.device.destroy_pipeline(self.graphics_pipeline, None);
            self.device.destroy_shader_module(self.fragment_shader_module, None);
            self.device.destroy_shader_module(self.vertex_shader_module, None);

            self.device.free_memory(self.index_buffer_memory, None);
            self.device.destroy_buffer(self.index_buffer, None);

            self.device.free_memory(self.vertex_input_buffer_memory, None);
            self.device.destroy_buffer(self.vertex_input_buffer, None);

            for framebuffer in self.framebuffers.iter() {
                self.device.destroy_framebuffer(*framebuffer, None);
            }

            self.device.destroy_pipeline_layout(self.pipeline_layout, None);

            self.device.destroy_render_pass(self.render_pass, None);

            for i in 0..self.framebuffers.len() {
                self.device.destroy_semaphore(self.image_available_semaphores[i], None);
                self.device.destroy_semaphore(self.rendering_complete_semaphores[i], None);

                self.device.destroy_fence(self.frames_in_flight_fences[i], None);

                self.device.free_memory(self.depth_image_memories[i], None);
                self.device.destroy_image_view(self.depth_image_views[i], None);
                self.device.destroy_image(self.depth_images[i], None);
            }


        for &image_view in self.present_image_views.iter() {
                self.device.destroy_image_view(image_view, None);
            }

            self.device.destroy_command_pool(self.pool, None);
            self.device.destroy_fence(self.setup_commands_reuse_fence, None);

            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
            self.device.destroy_device(None);
        }
    }
}
