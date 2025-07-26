mod img;
mod instance;
mod physical_surface;
mod device;
mod swapchain;
mod command_buffer;
mod framebuffer;
mod render_pass;
pub mod render_object;
mod buffer;
pub mod utils;
mod physical_device;
mod texture;

pub use instance::*;
use std::ops::Drop;
use ash::vk;
use ash::khr::{swapchain as ash_swapchain};

use winit::raw_window_handle::{HasDisplayHandle, HasRawDisplayHandle, HasWindowHandle};
use std::io::Cursor;
use std::mem::ManuallyDrop;

use std::sync::Arc;
use std::time::SystemTime;
use ash::vk::{DescriptorType, DeviceSize, ShaderStageFlags};
use async_trait::async_trait;
use cgmath::num_traits::FloatConst;
use parking_lot::Mutex;
use winit::event::WindowEvent;
use winit::window::Window;
use render_object::Vertex;

use slang_refl;

use crate::render_backend;
use render_backend::RenderBackend;
use render_backend::vulkan::buffer::GpuBuffer;
use render_backend::vulkan::command_buffer::{CommandBuffers, OneshotCommandBuffer};
use render_backend::vulkan::device::Device;
use render_backend::vulkan::framebuffer::Framebuffer;
use render_backend::vulkan::physical_surface::PhysicalSurface;
use render_backend::vulkan::swapchain::Swapchain;
use render_backend::vulkan::render_object::UniformBufferObject;
use render_backend::vulkan::render_pass::RenderPass;

use image;
use wgpu::MemoryHints::Manual;
use crate::render_backend::vulkan::img::Image;
use crate::render_backend::vulkan::texture::Texture;

#[allow(clippy::too_many_arguments)]
pub fn record_submit_commandbuffer<F: FnOnce(&ash::Device, vk::CommandBuffer)>(
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    in_flight_fence: &vk::Fence,
    submit_queue: vk::Queue,
    wait_mask: &[vk::PipelineStageFlags],
    wait_semaphores: &[vk::Semaphore],
    signal_semaphores: &[vk::Semaphore],
    f: F,
) {
    unsafe {
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
            .queue_submit(submit_queue, &[submit_info], *in_flight_fence)
            .expect("queue submit failed.");
    }
}


/// Vulkan Context which contains physical device, logical device, and surface, etc.
/// There will probably be a pointer of this being passed around
pub struct VulkanRenderBackend {
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

    pub vertex_buffer: ManuallyDrop<GpuBuffer<Vertex>>,
    pub index_buffer: ManuallyDrop<GpuBuffer<u32>>,
    pub texture: ManuallyDrop<Texture>,

    pub uniform_buffers: ManuallyDrop<Vec<GpuBuffer<UniformBufferObject>>>,

    pub triangle_shader_module: vk::ShaderModule,

    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub pipeline_layout: vk::PipelineLayout,

    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_sets: Vec<vk::DescriptorSet>,
    pub current_frame: usize,
}

impl VulkanRenderBackend { 
    fn update_uniform_buffer(vulkan_context: &mut VulkanRenderBackend, current_frame: usize, elapsed_time: f32) {
        let res = vulkan_context.physical_surface.surface_resolution;
        let aspect = res.width as f32 / res.height as f32;
        let new_ubo = [UniformBufferObject {
            model: cgmath::Matrix4::from_angle_z(cgmath::Deg(90.0 * elapsed_time)),
            view: cgmath::Matrix4::look_at(
                cgmath::Point3::new(2.0, 2.0, 2.0),
                cgmath::Point3::new(0.0, 0.0, 0.0),
                cgmath::Vector3::new(0.0, 0.0, 1.0),
            ),
            proj: utils::perspective(cgmath::Deg(45.0), aspect, 0.1, 10.0),
        }];
        vulkan_context.uniform_buffers[current_frame].write(&new_ubo);
    }
    
    pub unsafe fn recreate_swapchain(self: &mut Self, surface_size: vk::Extent2D) {
        self.device.device.device_wait_idle().unwrap();

        self.physical_surface.update_resolution(surface_size);
        
        ManuallyDrop::drop(&mut self.swapchain);
        ManuallyDrop::drop(&mut self.framebuffers);
        

        self.swapchain = ManuallyDrop::new(Swapchain::new(
            &self.instance, &self.physical_surface, &self.device));

        let mut framebuffers = Vec::new();
        for &swapchain_image_view in self.swapchain.image_views.iter() {
            let framebuffer = Framebuffer::new(&self.device, &self.render_pass,
                swapchain_image_view, self.physical_surface.resolution());
            framebuffers.push(framebuffer);
        }
        self.framebuffers = ManuallyDrop::new(framebuffers);
    }
}

impl VulkanRenderBackend {
    pub(crate) fn new(window: &Arc<dyn Window>) -> Self {
        let instance =
            ManuallyDrop::new(Instance::new(window));

        let physical_surface =
            ManuallyDrop::new(PhysicalSurface::new(&instance, window));

        let device =
            ManuallyDrop::new(Arc::new(Device::new(&instance, &physical_surface)));

        let swapchain = ManuallyDrop::new(Swapchain::new(
            &instance, &physical_surface, &device
        ));

        let draw_command_buffers =
            device.spawn_command_buffers(swapchain.images_count().try_into().unwrap());


        let semaphore_create_info = vk::SemaphoreCreateInfo::default();
        let fence_create_info = vk::FenceCreateInfo::default()
            .flags(vk::FenceCreateFlags::SIGNALED);

        let mut frames_in_flight_fences = Vec::new();
        let mut image_available_semaphores = Vec::new();
        let mut rendering_complete_semaphores = Vec::new();

        unsafe {
            for _ in 0..swapchain.images_count() {
                let fence = device.device.create_fence(&fence_create_info, None).unwrap();
                frames_in_flight_fences.push(fence);
                let image_available_semaphore = device.device.create_semaphore(&semaphore_create_info, None).unwrap();
                let rendering_complete_semaphore = device.device.create_semaphore(&semaphore_create_info, None).unwrap();
                image_available_semaphores.push(image_available_semaphore);
                rendering_complete_semaphores.push(rendering_complete_semaphore);
            }
        }

        let render_pass = ManuallyDrop::new(
            RenderPass::new(&physical_surface, &device));


        let mut framebuffers = Vec::new();
        for &swapchain_image_view in swapchain.image_views.iter() {
            let framebuffer =
                Framebuffer::new(
                    &device,
                    &render_pass,
                    swapchain_image_view,
                    physical_surface.resolution());
            framebuffers.push(framebuffer);
        }

        let framebuffers = ManuallyDrop::new(framebuffers);

        let index_buffer_data = [0u32, 1, 2, 2, 3, 0];

        let mut index_buffer = ManuallyDrop::new(GpuBuffer::<u32>::create_buffer(
            &device,
            vk::BufferUsageFlags::INDEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE
                | vk::MemoryPropertyFlags::HOST_COHERENT,
            index_buffer_data.len()
        ));

        index_buffer.map();
        index_buffer.write(&index_buffer_data);
        index_buffer.unmap();

        let vertices = vec!{
            Vertex {pos: [-0.5f32, -0.5f32, 0.0f32],  color: [1.0f32, 0.0f32, 0.0f32], tex_coord: [1.0f32, 0.0f32]},
            Vertex {pos: [0.5f32, -0.5f32, 0.0f32],  color: [0.0f32, 1.0f32, 0.0f32], tex_coord: [0.0f32, 0.0f32]},
            Vertex {pos: [0.5f32, 0.5f32, 0.0f32],  color: [0.0f32, 0.0f32, 1.0f32], tex_coord: [0.0f32, 1.0f32]},
            Vertex {pos: [-0.5f32, 0.5f32, 0.0f32],  color: [1.0f32, 1.0f32, 1.0f32], tex_coord: [1.0f32, 1.0f32]},
        };

        let mut vertex_buffer = ManuallyDrop::new(GpuBuffer::<Vertex>::create_buffer(
            &device,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE
                | vk::MemoryPropertyFlags::HOST_COHERENT,
            vertices.len()));
        vertex_buffer.map();
        vertex_buffer.write(&vertices);
        vertex_buffer.unmap();

        let diffuse_bytes = include_bytes!("../../../../textures/happy-tree.png");
        let img = image::load_from_memory(diffuse_bytes).unwrap();

        let texture_image = Texture::from_image(&device, img.to_rgba8());
        let texture_image = ManuallyDrop::new(texture_image);

        let ubo_layout_binding = vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(ShaderStageFlags::VERTEX);
        let sampler_layout_binding = vk::DescriptorSetLayoutBinding::default()
            .binding(1)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT);

        let bindings = [
            ubo_layout_binding, sampler_layout_binding];

        let descriptor_set_layout_create_info =
            vk::DescriptorSetLayoutCreateInfo::default()
                .bindings(&bindings);

        let descriptor_set_layout = unsafe {
            device.device.create_descriptor_set_layout(&descriptor_set_layout_create_info, None).unwrap()
        };

        let mut uniform_buffers = Vec::new();
        for _ in 0..framebuffers.len() {
            let mut ubo_buffer = GpuBuffer::<UniformBufferObject>::create_buffer(
                &device,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE
                    | vk::MemoryPropertyFlags::HOST_COHERENT,
                1);

            ubo_buffer.map();
            uniform_buffers.push(ubo_buffer);
        }

        let uniform_buffers = ManuallyDrop::new(uniform_buffers);

        let descriptor_pool_sizes = [
            vk::DescriptorPoolSize::default()
                .descriptor_count(framebuffers.len() as u32)
                .ty(DescriptorType::UNIFORM_BUFFER),
            vk::DescriptorPoolSize::default()
                .descriptor_count(framebuffers.len() as u32)
                .ty(DescriptorType::COMBINED_IMAGE_SAMPLER)];

        let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&descriptor_pool_sizes)
            .max_sets(framebuffers.len() as u32);

        let descriptor_pool = unsafe {
            device.device.create_descriptor_pool(&descriptor_pool_create_info, None).unwrap()
        };

        let descriptor_set_layouts = vec![descriptor_set_layout; framebuffers.len()];
        let descriptor_set_alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&descriptor_set_layouts);

        let descriptor_sets = unsafe {
            device.device.allocate_descriptor_sets(&descriptor_set_alloc_info).unwrap()
        };

        for i in 0..framebuffers.len() {
            let descriptor_buffer_info = [vk::DescriptorBufferInfo::default()
                .buffer(uniform_buffers[i].buffer).range(size_of::<UniformBufferObject>() as DeviceSize)];
            let descriptor_image_info = [vk::DescriptorImageInfo::default()
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image_view(texture_image.image.view)
                .sampler(texture_image.sampler)];
            let descriptor_write = [
                vk::WriteDescriptorSet::default()
                    .descriptor_count(1)
                    .descriptor_type(DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(&descriptor_buffer_info)
                    .dst_binding(0)
                    .dst_array_element(0)
                    .dst_set(descriptor_sets[i]),
                vk::WriteDescriptorSet::default()
                    .descriptor_count(1)
                    .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(&descriptor_image_info)
                    .dst_binding(1)
                    .dst_array_element(0)
                    .dst_set(descriptor_sets[i])
            ];
            unsafe {
                device.device.update_descriptor_sets(&descriptor_write, &[]);
            }
        }
        
        let compiler = slang_refl::Compiler::new();
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

        let layout_create_info =
            vk::PipelineLayoutCreateInfo::default().set_layouts(&descriptor_set_layouts);

        let pipeline_layout = unsafe { device.device
            .create_pipeline_layout(&layout_create_info, None)
            .unwrap() };

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
            vk::VertexInputAttributeDescription {
                location: 2,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: std::mem::offset_of!(Vertex, tex_coord) as u32,
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
            cull_mode: vk::CullModeFlags::BACK,
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

        let graphics_pipelines = unsafe { device.device
            .create_graphics_pipelines(vk::PipelineCache::null(), &[graphic_pipeline_info], None)
            .expect("Unable to create graphics pipeline")
        };

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
            vertex_buffer,
            index_buffer,
            uniform_buffers,
            texture: texture_image,

            triangle_shader_module,
            descriptor_set_layout,
            pipeline_layout,
            descriptor_pool,
            descriptor_sets,
            current_frame: 0
        }
    }
}

impl RenderBackend for VulkanRenderBackend {
    fn init(backend_to_init: Arc<Mutex<Option<Self>>>, window: Arc<dyn Window>) {
        let render_backend = VulkanRenderBackend::new(&window);
        backend_to_init.lock().replace(render_backend);
    }

    fn render(self: &mut Self) {
        unsafe {
            self.device.device.wait_for_fences(
                &[self.frames_in_flight_fences[self.current_frame]],
                true, u64::MAX).unwrap();
        }

        if let Some(extent) = self.new_swapchain_size {
            if extent.width <= 0 || extent.height <= 0 {
                return;
            }
            unsafe { self.recreate_swapchain(extent)};
            self.new_swapchain_size = None;
        }

        let viewports = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: self.physical_surface.resolution().width as f32,
            height: self.physical_surface.resolution().height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];
        let scissors = [self.physical_surface.resolution().into()];

        unsafe {self.device.device.reset_fences(&[self.frames_in_flight_fences[self.current_frame]]).unwrap()};

        let acquire_result = unsafe {self
            .swapchain.swapchain_loader
            .acquire_next_image(
                self.swapchain.swapchain,
                u64::MAX,
                self.image_available_semaphores[self.current_frame],
                vk::Fence::null())};

        let present_index = match acquire_result {
            Ok((present_index, _)) => present_index,
            _ => panic!("Failed to acquire swapchain."),
        };


        let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 0.0],
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            },
        ];

        let render_pass_begin_info = vk::RenderPassBeginInfo::default()
            .render_pass(self.render_pass.render_pass)
            .framebuffer(self.framebuffers[present_index as usize].framebuffer)
            .render_area(self.physical_surface.resolution().into())
            .clear_values(&clear_values);

        //let elapsed_time = SystemTime::now().duration_since(SystemTime::now()).unwrap().as_secs_f32();
        let elapsed_time = self.current_frame as f32 * 0.02;

        Self::update_uniform_buffer(self, self.current_frame, elapsed_time);

        record_submit_commandbuffer(
            &self.device.device,
            self.draw_command_buffers.command_buffers[self.current_frame],
            &self.frames_in_flight_fences[self.current_frame],
            self.device.present_queue,
            &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT],
            &[self.image_available_semaphores[self.current_frame]],
            &[self.rendering_complete_semaphores[present_index as usize]],
            |device, draw_command_buffer| {
                unsafe { device.cmd_begin_render_pass(
                    draw_command_buffer,
                    &render_pass_begin_info,
                    vk::SubpassContents::INLINE,
                );
                    device.cmd_bind_pipeline(
                        draw_command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.graphics_pipeline,
                    );
                    device.cmd_set_viewport(draw_command_buffer, 0, &viewports);
                    device.cmd_set_scissor(draw_command_buffer, 0, &scissors);
                    device.cmd_bind_vertex_buffers(
                        draw_command_buffer,
                        0,
                        &[self.vertex_buffer.buffer],
                        &[0],
                    );
                    device.cmd_bind_index_buffer(
                        draw_command_buffer,
                        self.index_buffer.buffer,
                        0,
                        vk::IndexType::UINT32,
                    );
                    device.cmd_bind_descriptor_sets(
                        draw_command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.pipeline_layout,
                        0, self.descriptor_sets[self.current_frame..self.current_frame+1].as_ref(), 
                        &[]);
                    device.cmd_draw_indexed(
                        draw_command_buffer,
                        6,
                        1,
                        0,
                        0,
                        1,
                    );
                    // Or draw without the index buffer
                    // device.cmd_draw(draw_command_buffer, 3, 1, 0, 0);
                    device.cmd_end_render_pass(draw_command_buffer);}
            },
        );
        let signal_semaphores = [self.rendering_complete_semaphores[present_index as usize]];
        let swapchains = [self.swapchain.swapchain];
        let image_indices = [present_index];
        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&signal_semaphores) // &base.rendering_complete_semaphore)
            .swapchains(&swapchains)
            .image_indices(&image_indices);


        unsafe {
            self.swapchain.swapchain_loader
                .queue_present(self.device.present_queue, &present_info)
                .unwrap()};
        
        self.current_frame += 1;
        self.current_frame = self.current_frame % 3;
    }

    fn process_event(self: &mut Self, event: &WindowEvent) {
        println!("process event");
    }

    fn resize(self: &mut Self, width: u32, height: u32) {
        self.new_swapchain_size = Some(vk::Extent2D { width, height });
    }

}

impl Drop for VulkanRenderBackend {
    fn drop(&mut self) {
        println!("Dropping application.");
        unsafe {
            self.device.device.device_wait_idle().unwrap();

            self.device.device.destroy_pipeline(self.graphics_pipeline, None);
            self.device.device.destroy_shader_module(self.triangle_shader_module, None);

            self.device.device.destroy_pipeline_layout(self.pipeline_layout, None);

            for i in 0..self.framebuffers.len() {
                self.device.device.destroy_semaphore(self.image_available_semaphores[i], None);
                self.device.device.destroy_semaphore(self.rendering_complete_semaphores[i], None);

                self.device.device.destroy_fence(self.frames_in_flight_fences[i], None);
            }

            self.device.device.destroy_descriptor_pool(self.descriptor_pool, None);
            self.device.device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);

            ManuallyDrop::drop(&mut self.framebuffers);
            ManuallyDrop::drop(&mut self.render_pass);
            ManuallyDrop::drop(&mut self.swapchain);
            ManuallyDrop::drop(&mut self.vertex_buffer);
            ManuallyDrop::drop(&mut self.index_buffer);
            ManuallyDrop::drop(&mut self.uniform_buffers);
            ManuallyDrop::drop(&mut self.texture);
            ManuallyDrop::drop(&mut self.device);
            ManuallyDrop::drop(&mut self.physical_surface);
            ManuallyDrop::drop(&mut self.instance);
        }
    }
}
