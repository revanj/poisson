mod image;
mod instance;
mod physical_surface;
mod device;
mod swapchain;
mod command_buffer;

pub use instance::*;
use std::ops::Drop;
use ash::vk;
use ash::khr::{swapchain as ash_swapchain};

use ash::vk::{CommandBuffer, DebugUtilsMessengerEXT, PhysicalDevice};
use winit::raw_window_handle::{HasDisplayHandle, HasRawDisplayHandle, HasWindowHandle, RawDisplayHandle};
use std::ffi::c_char;
use std::io::Cursor;
use std::mem::ManuallyDrop;
use ash::util::read_spv;
use ash_window;
use winit::window::Window;
use crate::Vertex;
use crate::vulkan::command_buffer::{CommandBuffers, OneshotCommandBuffer};
use crate::vulkan::device::Device;
use crate::vulkan::physical_surface::PhysicalSurface;
use crate::vulkan::swapchain::Swapchain;
use crate::vulkan::image::Image;

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
    pub instance: ManuallyDrop<Instance>,
    pub physical_surface: ManuallyDrop<PhysicalSurface>,
    pub device : ManuallyDrop<Device>,
    pub swapchain: ManuallyDrop<Swapchain>,

    pub new_swapchain_size: Option<vk::Extent2D>,

    pub draw_command_buffers: CommandBuffers,

    pub depth_images: Vec<Image>,

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
        let instance =
            ManuallyDrop::new(Instance::new(window));

        let physical_surface =
            ManuallyDrop::new(PhysicalSurface::new(&instance, window));

        let device = ManuallyDrop::new(Device::new(
            &instance, &physical_surface));

        let swapchain = ManuallyDrop::new(Swapchain::new(
            &instance, &physical_surface, &device
        ));


        let draw_command_buffers =
            device.spawn_command_buffers(swapchain.images_count().try_into().unwrap());



        let mut depth_images = Vec::new();

        for _ in 0..swapchain.images_count() {
            let depth_image = Image::new_depth_image(&device, physical_surface.surface_resolution);
            depth_images.push(depth_image);
        }

        let semaphore_create_info = vk::SemaphoreCreateInfo::default();
        let fence_create_info = vk::FenceCreateInfo::default()
            .flags(vk::FenceCreateFlags::SIGNALED);

        let mut frames_in_flight_fences = Vec::new();
        let mut image_available_semaphores = Vec::new();
        let mut rendering_complete_semaphores = Vec::new();

        for _ in 0..swapchain.images_count() {
            let fence = device.device.create_fence(&fence_create_info, None).unwrap();
            frames_in_flight_fences.push(fence);
            let image_available_semaphore = device.device.create_semaphore(&semaphore_create_info, None).unwrap();
            let rendering_complete_semaphore = device.device.create_semaphore(&semaphore_create_info, None).unwrap();
            image_available_semaphores.push(image_available_semaphore);
            rendering_complete_semaphores.push(rendering_complete_semaphore);
        }

        let renderpass_attachments = [
            vk::AttachmentDescription {
                format: physical_surface.surface_format.format,
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

        let render_pass = device.device
            .create_render_pass(&renderpass_create_info, None)
            .unwrap();

        let framebuffers: Vec<vk::Framebuffer> = swapchain.image_views.iter().enumerate()
            .map(|(index, &present_image_view)| {
                let framebuffer_attachments = [present_image_view, depth_images[index].view];
                let frame_buffer_create_info = vk::FramebufferCreateInfo::default()
                    .render_pass(render_pass)
                    .attachments(&framebuffer_attachments)
                    .width(physical_surface.surface_resolution.width)
                    .height(physical_surface.surface_resolution.height)
                    .layers(1);

                device.device
                    .create_framebuffer(&frame_buffer_create_info, None)
                    .unwrap()
            })
            .collect();

        let index_buffer_data = [0u32, 1, 2];
        let index_buffer_info = vk::BufferCreateInfo::default()
            .size(size_of_val(&index_buffer_data) as u64)
            .usage(vk::BufferUsageFlags::INDEX_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let index_buffer = device.device.create_buffer(&index_buffer_info, None).unwrap();
        let index_buffer_memory_req = device.device.get_buffer_memory_requirements(index_buffer);
        let index_buffer_memory_index = find_memorytype_index(
            &index_buffer_memory_req,
            &device.physical_memory_properties,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )
            .expect("Unable to find suitable memorytype for the index buffer.");

        let index_allocate_info = vk::MemoryAllocateInfo {
            allocation_size: index_buffer_memory_req.size,
            memory_type_index: index_buffer_memory_index,
            ..Default::default()
        };
        let index_buffer_memory = device.device
            .allocate_memory(&index_allocate_info, None)
            .unwrap();
        let index_ptr = device.device
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
        device.device.unmap_memory(index_buffer_memory);
        device.device
            .bind_buffer_memory(index_buffer, index_buffer_memory, 0)
            .unwrap();

        let vertex_input_buffer_info = vk::BufferCreateInfo {
            size: 3 * size_of::<Vertex>() as u64,
            usage: vk::BufferUsageFlags::VERTEX_BUFFER,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let vertex_input_buffer = device.device
            .create_buffer(&vertex_input_buffer_info, None)
            .unwrap();

        let vertex_input_buffer_memory_req = device.device
            .get_buffer_memory_requirements(vertex_input_buffer);

        let vertex_input_buffer_memory_index = find_memorytype_index(
            &vertex_input_buffer_memory_req,
            &device.physical_memory_properties,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )
            .expect("Unable to find suitable memorytype for the vertex buffer.");

        let vertex_buffer_allocate_info = vk::MemoryAllocateInfo {
            allocation_size: vertex_input_buffer_memory_req.size,
            memory_type_index: vertex_input_buffer_memory_index,
            ..Default::default()
        };

        let vertex_input_buffer_memory = device.device
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

        let vert_ptr = device.device
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
        device.device.unmap_memory(vertex_input_buffer_memory);
        device.device
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

        let vertex_shader_module = device.device
            .create_shader_module(&vertex_shader_info, None)
            .expect("Vertex shader module error");

        let fragment_shader_module = device.device
            .create_shader_module(&frag_shader_info, None)
            .expect("Fragment shader module error");

        let layout_create_info = vk::PipelineLayoutCreateInfo::default();

        let pipeline_layout = device.device
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
            width: physical_surface.surface_resolution.width as f32,
            height: physical_surface.surface_resolution.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];
        let scissors = [physical_surface.surface_resolution.into()];
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

        let graphics_pipelines = device.device
            .create_graphics_pipelines(vk::PipelineCache::null(), &[graphic_pipeline_info], None)
            .expect("Unable to create graphics pipeline");

        let graphics_pipeline = graphics_pipelines[0];



        Self {
            instance,
            physical_surface,
            device,
            swapchain,
            new_swapchain_size: None,

            draw_command_buffers,

            depth_images,

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
        self.device.device.device_wait_idle().unwrap();

        for framebuffer in self.framebuffers.iter() {
            self.device.device.destroy_framebuffer(*framebuffer, None);
        }

        for i in 0..self.framebuffers.len() {
            self.device.device.free_memory(self.depth_images[i].memory, None);
            self.device.device.destroy_image_view(self.depth_images[i].view, None);
            self.device.device.destroy_image(self.depth_images[i].image, None);
        }


        for &image_view in self.swapchain.image_views.iter() {
            self.device.device.destroy_image_view(image_view, None);
        }


        self.swapchain.swapchain_loader.destroy_swapchain(self.swapchain.swapchain, None);

        let surface_capabilities = self.physical_surface.surface_loader
            .get_physical_device_surface_capabilities(self.physical_surface.physical_device, self.physical_surface.surface)
            .unwrap();

        let mut desired_image_count = surface_capabilities.min_image_count + 1;
        if surface_capabilities.max_image_count > 0
            && desired_image_count > surface_capabilities.max_image_count
        {
            desired_image_count = surface_capabilities.max_image_count;
        }

        self.physical_surface.surface_resolution = match surface_capabilities.current_extent.width {
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

        self.swapchain.swapchain_loader = ash_swapchain::Device::new(&self.instance.instance, &self.device.device);

        let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(self.physical_surface.surface)
            .min_image_count(desired_image_count)
            .image_color_space(self.physical_surface.surface_format.color_space)
            .image_format(self.physical_surface.surface_format.format)
            .image_extent(self.physical_surface.surface_resolution)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(pre_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .image_array_layers(1);

        self.swapchain.swapchain = self.swapchain.swapchain_loader
            .create_swapchain(&swapchain_create_info, None)
            .unwrap();

        self.swapchain.images = self.swapchain.swapchain_loader.get_swapchain_images(self.swapchain.swapchain).unwrap();
        self.swapchain.image_views = self.swapchain.images
            .iter()
            .map(|&image| {
                let create_view_info = vk::ImageViewCreateInfo::default()
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(self.physical_surface.surface_format.format)
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
                self.device.device.create_image_view(&create_view_info, None).unwrap()
            })
            .collect();

        let n_frame_buffers = self.swapchain.images_count();

        self.depth_images = Vec::new();
        for _ in 0..n_frame_buffers {
            let depth_image = Image::new_depth_image(&self.device, self.physical_surface.surface_resolution);
            self.depth_images.push(depth_image);
        }

        self.framebuffers = self.swapchain.image_views
            .iter().enumerate()
            .map(|(index, &present_image_view)| {
                let framebuffer_attachments = [present_image_view, self.depth_images[index].view];
                let frame_buffer_create_info = vk::FramebufferCreateInfo::default()
                    .render_pass(self.render_pass)
                    .attachments(&framebuffer_attachments)
                    .width(self.physical_surface.surface_resolution.width)
                    .height(self.physical_surface.surface_resolution.height)
                    .layers(1);

                self.device.device
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
            self.device.device.device_wait_idle().unwrap();

            self.device.device.destroy_pipeline(self.graphics_pipeline, None);
            self.device.device.destroy_shader_module(self.fragment_shader_module, None);
            self.device.device.destroy_shader_module(self.vertex_shader_module, None);

            self.device.device.free_memory(self.index_buffer_memory, None);
            self.device.device.destroy_buffer(self.index_buffer, None);

            self.device.device.free_memory(self.vertex_input_buffer_memory, None);
            self.device.device.destroy_buffer(self.vertex_input_buffer, None);

            for framebuffer in self.framebuffers.iter() {
                self.device.device.destroy_framebuffer(*framebuffer, None);
            }

            self.device.device.destroy_pipeline_layout(self.pipeline_layout, None);

            self.device.device.destroy_render_pass(self.render_pass, None);

            for i in 0..self.framebuffers.len() {
                self.device.device.destroy_semaphore(self.image_available_semaphores[i], None);
                self.device.device.destroy_semaphore(self.rendering_complete_semaphores[i], None);

                self.device.device.destroy_fence(self.frames_in_flight_fences[i], None);

                self.device.device.free_memory(self.depth_images[i].memory, None);
                self.device.device.destroy_image_view(self.depth_images[i].view, None);
                self.device.device.destroy_image(self.depth_images[i].image, None);
            }

            for &image_view in self.swapchain.image_views.iter() {
                self.device.device.destroy_image_view(image_view, None);
            }

            ManuallyDrop::drop(&mut self.swapchain);
            ManuallyDrop::drop(&mut self.device);
            ManuallyDrop::drop(&mut self.physical_surface);
            ManuallyDrop::drop(&mut self.instance);
        }
    }
}
