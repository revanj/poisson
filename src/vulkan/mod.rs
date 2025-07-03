mod image;
mod instance;
mod physical_surface;
mod device;
mod swapchain;
mod command_buffer;
mod framebuffer;
mod render_pass;
pub mod render_object;
mod buffer;
mod utils;

pub use instance::*;
use std::ops::Drop;
use ash::vk;
use ash::khr::{swapchain as ash_swapchain};

use winit::raw_window_handle::{HasDisplayHandle, HasRawDisplayHandle, HasWindowHandle};
use std::io::Cursor;
use std::mem::ManuallyDrop;

use std::sync::Arc;
use ash::util::read_spv;
use ash_window;
use winit::window::Window;
use render_object::Vertex;
use crate::slang;
use crate::vulkan::command_buffer::{CommandBuffers, OneshotCommandBuffer};
use crate::vulkan::device::Device;
use crate::vulkan::framebuffer::Framebuffer;
use crate::vulkan::physical_surface::PhysicalSurface;
use crate::vulkan::swapchain::Swapchain;
use crate::vulkan::image::Image;
use crate::vulkan::render_pass::RenderPass;


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
    pub device : ManuallyDrop<Arc<Device>>,
    pub swapchain: ManuallyDrop<Swapchain>,
    pub new_swapchain_size: Option<vk::Extent2D>,

    pub render_pass: ManuallyDrop<RenderPass>,
    pub framebuffers: ManuallyDrop<Vec<Framebuffer>>,

    pub draw_command_buffers: CommandBuffers,


    pub image_available_semaphores: Vec<vk::Semaphore>,
    pub rendering_complete_semaphores: Vec<vk::Semaphore>,
    pub frames_in_flight_fences: Vec<vk::Fence>,


    pub graphics_pipeline: vk::Pipeline,
    pub vertex_input_buffer: vk::Buffer,
    pub vertex_input_buffer_memory: vk::DeviceMemory,
    pub index_buffer: vk::Buffer,
    pub index_buffer_memory: vk::DeviceMemory,

    pub triangle_shader_module: vk::ShaderModule,
    pub pipeline_layout: vk::PipelineLayout,
}

impl VulkanContext {

    pub unsafe fn new(window: &Box<dyn Window>) -> Self {
        let instance =
            ManuallyDrop::new(Instance::new(window));

        let physical_surface =
            ManuallyDrop::new(PhysicalSurface::new(&instance, window));

        let device =
            ManuallyDrop::new(Arc::new(
                Device::new(
                    &instance, &physical_surface
                )));

        let swapchain = ManuallyDrop::new(Swapchain::new(
            &instance, &physical_surface, &*device
        ));
        
        let draw_command_buffers =
            device.spawn_command_buffers(swapchain.images_count().try_into().unwrap());


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

        let render_pass = ManuallyDrop::new(
            RenderPass::new(&physical_surface, &device));


        let mut framebuffers = Vec::new();
        for &swapchain_image_view in swapchain.image_views.iter() {
            let framebuffer =
                Framebuffer::new(
                    &*device,
                    &render_pass,
                    swapchain_image_view,
                    physical_surface.resolution());
            framebuffers.push(framebuffer);
        }

        let framebuffers = ManuallyDrop::new(framebuffers);

        let index_buffer_data = [0u32, 1, 2];
        let index_buffer_info = vk::BufferCreateInfo::default()
            .size(size_of_val(&index_buffer_data) as u64)
            .usage(vk::BufferUsageFlags::INDEX_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let index_buffer = device.device.create_buffer(&index_buffer_info, None).unwrap();
        let index_buffer_memory_req = device.device.get_buffer_memory_requirements(index_buffer);
        let index_buffer_memory_index = utils::find_memorytype_index(
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

        let vertex_input_buffer_memory_index = utils::find_memorytype_index(
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
                pos: [-1.0, 1.0, 0.0],
                color: [0.0, 1.0, 0.0],
            },
            Vertex {
                pos: [1.0, 1.0, 0.0],
                color: [0.0, 0.0, 1.0],
            },
            Vertex {
                pos: [0.0, -1.0, 0.0],
                color: [1.0, 0.0, 0.0],
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

        let compiler = slang::Compiler::new();
        let linked_program = compiler.linked_program_from_file("shaders/triangle.slang");

        let refl = linked_program.get_reflection();

        println!("there are {} entry points in shader", refl.entry_point_reflections.len());
        for entry in refl.entry_point_reflections {
            println!("{:?} shader {}(), with fields", entry.stage, entry.name);
            for s in entry.struct_reflections {
                println!("\t{}", s);
            }
        }

        let compiled_triangle_shader = linked_program.get_bytecode();

        let triangle_shader_info = vk::ShaderModuleCreateInfo::default().code(&compiled_triangle_shader);
        let triangle_shader_module = unsafe { device.device.create_shader_module(&triangle_shader_info, None) }
            .expect("Vertex shader module error");

        let layout_create_info = vk::PipelineLayoutCreateInfo::default();

        let pipeline_layout = device.device
            .create_pipeline_layout(&layout_create_info, None)
            .unwrap();

        let vertex_entry_name = c"vertexMain";
        let fragment_entry_name = c"fragmentMain";
        let shader_stage_create_infos = [
            vk::PipelineShaderStageCreateInfo {
                module: triangle_shader_module,
                p_name: vertex_entry_name.as_ptr(),
                stage: vk::ShaderStageFlags::VERTEX,
                ..Default::default()
            },
            vk::PipelineShaderStageCreateInfo {
                s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
                module: triangle_shader_module,
                p_name: fragment_entry_name.as_ptr(),
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
                format: vk::Format::R32G32B32_SFLOAT,
                offset: std::mem::offset_of!(Vertex, pos) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: vk::Format::R32G32B32_SFLOAT,
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

        let resolution = physical_surface.resolution();
        let viewports = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: resolution.width as f32,
            height: resolution.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];
        let scissors = [resolution.into()];
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
            .render_pass(render_pass.render_pass);

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

            render_pass,
            framebuffers,

            draw_command_buffers,

            image_available_semaphores,
            rendering_complete_semaphores,
            frames_in_flight_fences,

            graphics_pipeline,
            vertex_input_buffer,
            vertex_input_buffer_memory,
            index_buffer,
            index_buffer_memory,
            triangle_shader_module,
            pipeline_layout
        }
    }

    pub unsafe fn recreate_swapchain(self: &mut Self, surface_size: vk::Extent2D) {
        self.device.device.device_wait_idle().unwrap();

        self.physical_surface.update_resolution(surface_size);

        ManuallyDrop::drop(&mut self.framebuffers);
        ManuallyDrop::drop(&mut self.swapchain);

        self.swapchain = ManuallyDrop::new(Swapchain::new(
            &self.instance, &self.physical_surface, &self.device));

        let mut framebuffers = Vec::new();
        for &swapchain_image_view in self.swapchain.image_views.iter() {
            let framebuffer = Framebuffer::new(&*self.device, &self.render_pass,
                swapchain_image_view, self.physical_surface.resolution());
            framebuffers.push(framebuffer);
        }
        self.framebuffers = ManuallyDrop::new(framebuffers);
    }
}

impl Drop for VulkanContext {
    fn drop(&mut self) {
        println!("Dropping application.");
        unsafe {
            self.device.device.device_wait_idle().unwrap();

            self.device.device.destroy_pipeline(self.graphics_pipeline, None);
            self.device.device.destroy_shader_module(self.triangle_shader_module, None);
            self.device.device.free_memory(self.index_buffer_memory, None);
            self.device.device.destroy_buffer(self.index_buffer, None);

            self.device.device.free_memory(self.vertex_input_buffer_memory, None);
            self.device.device.destroy_buffer(self.vertex_input_buffer, None);

            self.device.device.destroy_pipeline_layout(self.pipeline_layout, None);



            for i in 0..self.framebuffers.len() {
                self.device.device.destroy_semaphore(self.image_available_semaphores[i], None);
                self.device.device.destroy_semaphore(self.rendering_complete_semaphores[i], None);

                self.device.device.destroy_fence(self.frames_in_flight_fences[i], None);
            }

            ManuallyDrop::drop(&mut self.framebuffers);
            ManuallyDrop::drop(&mut self.render_pass);
            ManuallyDrop::drop(&mut self.swapchain);
            ManuallyDrop::drop(&mut self.device);
            ManuallyDrop::drop(&mut self.physical_surface);
            ManuallyDrop::drop(&mut self.instance);
        }
    }
}
