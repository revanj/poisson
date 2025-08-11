use crate::AsAny;
use std::any::{Any};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, Weak};
use ash::vk;
use ash::vk::{CommandBuffer, DescriptorSetLayout, DescriptorType, DeviceSize, Extent2D, Pipeline, ShaderStageFlags};

use image::{DynamicImage, RgbaImage};
use vk::PipelineLayout;
use poisson_macros::AsAny;
use crate::render_backend::{RenderDrawlet, RenderPipeline, TexturedMesh, TexturedMeshData, Mat4Ubo, Vertex, VulkanDrawlet, VulkanDrawletDyn, VulkanPipeline, VulkanPipelineDyn};
use crate::render_backend::{DrawletHandle, DrawletID, PipelineHandle, PipelineID, RenderBackend};
use crate::render_backend::vulkan::buffer::GpuBuffer;
use crate::render_backend::vulkan::device::Device;
use crate::render_backend::vulkan::render_pass::RenderPass;
use crate::render_backend::vulkan::texture::Texture;


#[derive(AsAny)]
pub struct TexturedMeshPipeline {
    device: Weak<Device>,
    pub pipeline: vk::Pipeline,
    shader_module: vk::ShaderModule,
    pub pipeline_layout: vk::PipelineLayout,
    descriptor_set_layout: DescriptorSetLayout,
    resolution: vk::Extent2D,
    n_framebuffers: usize,
    pub instances: HashMap<DrawletID, TexturedMeshDrawlet>,
}

// this can most certainly be turned into a macro
impl VulkanPipelineDyn for TexturedMeshPipeline {
    fn get_pipeline(self: &Self) -> Pipeline {
        self.pipeline
    }

    fn get_instances(self: &Self) -> Box<dyn Iterator<Item=&dyn VulkanDrawletDyn> + '_> {
        Box::new(self.instances.iter().map(|(_, x)| x as &dyn VulkanDrawletDyn))
    }

    fn get_instances_mut(self: &mut Self) -> Box<dyn Iterator<Item=&mut dyn VulkanDrawletDyn> + '_> {
        Box::new(self.instances.iter_mut().map(|(_, x)| x as &mut dyn VulkanDrawletDyn))
    }
}

impl RenderPipeline<TexturedMesh> for TexturedMeshPipeline {
    
}

impl VulkanPipeline<TexturedMesh> for TexturedMeshPipeline {
    
    fn instantiate_drawlet(self: &mut Self, init_data: TexturedMeshData) -> DrawletHandle<TexturedMesh> {
        let drawlet_id = Self::get_drawlet_id();
        self.instances.insert(drawlet_id, TexturedMeshDrawlet::new(
            &self.device.upgrade().unwrap(),
            &init_data.index_data,
            &init_data.vertex_data,
            &init_data.texture_data.as_rgba8().unwrap(),
            self.descriptor_set_layout,
            self.n_framebuffers,
            self.resolution,
            self.pipeline_layout));

        DrawletHandle::<TexturedMesh> {
            id: drawlet_id,
            _drawlet_ty: PhantomData::default(),
        }
    }

    fn get_drawlet_mut(&mut self, drawlet_handle: &DrawletHandle<TexturedMesh>) -> &'_ mut TexturedMeshDrawlet {
        self.instances.get_mut(&drawlet_handle.id).unwrap()
    }
    fn new(device: &Arc<Device>, render_pass: &RenderPass, shader_bytecode: &[u32], resolution: Extent2D, n_framebuffers: usize) -> Self {
        let ubo_layout_binding = vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(ShaderStageFlags::VERTEX);
        let texture_layout_binding = vk::DescriptorSetLayoutBinding::default()
            .binding(1)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT);
        let sampler_layout_binding = vk::DescriptorSetLayoutBinding::default()
            .binding(2)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT);

        let bindings = [
            ubo_layout_binding, texture_layout_binding, sampler_layout_binding];

        let descriptor_set_layout_create_info =
            vk::DescriptorSetLayoutCreateInfo::default()
                .bindings(&bindings);

        let descriptor_set_layout = unsafe {
            device.device.create_descriptor_set_layout(&descriptor_set_layout_create_info, None).unwrap()
        };
        let compiled_triangle_shader = shader_bytecode;

        let shader_info = vk::ShaderModuleCreateInfo::default().code(&compiled_triangle_shader);
        let shader_module = unsafe { device.device.create_shader_module(&shader_info, None) }
            .expect("Vertex shader module error");

        let descriptor_set_layouts = vec![descriptor_set_layout; n_framebuffers];
        let layout_create_info =
            vk::PipelineLayoutCreateInfo::default().set_layouts(&descriptor_set_layouts);

        let pipeline_layout = unsafe {
            device.device
                .create_pipeline_layout(&layout_create_info, None)
                .unwrap()
        };

        let vertex_entry_name = c"vertexMain";
        let fragment_entry_name = c"fragmentMain";
        let shader_stage_create_infos = [
            vk::PipelineShaderStageCreateInfo {
                module: shader_module,
                p_name: vertex_entry_name.as_ptr(),
                stage: vk::ShaderStageFlags::VERTEX,
                ..Default::default()
            },
            vk::PipelineShaderStageCreateInfo {
                s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
                module: shader_module,
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

        let graphics_pipelines = unsafe {
            device.device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[graphic_pipeline_info], None)
                .expect("Unable to create graphics pipeline")
        };

        let instances = HashMap::new();

        TexturedMeshPipeline {
            device: Arc::downgrade(device),
            pipeline: graphics_pipelines[0],
            descriptor_set_layout,
            shader_module,
            pipeline_layout,
            resolution,
            n_framebuffers,
            instances
        }
    }
}


impl Drop for TexturedMeshPipeline {
    fn drop(&mut self) {
        let device = self.device.upgrade().unwrap();
        unsafe {
            device.device.destroy_pipeline(self.pipeline, None);
            device.device.destroy_shader_module(self.shader_module, None);
            device.device.destroy_pipeline_layout(self.pipeline_layout, None);
            device.device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
    }
}
 
pub struct TexturedMeshDrawlet {
    pub device: Weak<Device>,
    pub index_buffer: GpuBuffer<u32>,
    pub vertex_buffer: GpuBuffer<Vertex>,
    pub texture: Texture,
    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_sets: Vec<vk::DescriptorSet>,
    pub uniform_buffers: Vec<GpuBuffer<Mat4Ubo>>,
    pub resolution: vk::Extent2D,
    pub pipeline_layout: PipelineLayout,
    pub current_frame: usize,
}


impl TexturedMeshDrawlet {
    pub fn new(
        device: &Arc<Device>,
        index_data: &[u32], 
        vertex_data: &[Vertex], 
        texture_data: &RgbaImage,
        descriptor_set_layout: DescriptorSetLayout,
        n_framebuffers: usize,
        resolution: vk::Extent2D,
        pipeline_layout: PipelineLayout) -> Self
    {
        let mut index_buffer = GpuBuffer::<u32>::create_buffer(
            &device,
            vk::BufferUsageFlags::INDEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE
                | vk::MemoryPropertyFlags::HOST_COHERENT,
            index_data.len()
        );
        index_buffer.map();
        index_buffer.write(&index_data);
        index_buffer.unmap();

        let mut vertex_buffer = GpuBuffer::<Vertex>::create_buffer(
            &device,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE
                | vk::MemoryPropertyFlags::HOST_COHERENT,
            vertex_data.len());
        vertex_buffer.map();
        vertex_buffer.write(&vertex_data);
        vertex_buffer.unmap();

        let texture = Texture::from_image(&device, texture_data);

        let descriptor_pool_sizes = [
            vk::DescriptorPoolSize::default()
                .descriptor_count(n_framebuffers as u32)
                .ty(DescriptorType::UNIFORM_BUFFER),
            vk::DescriptorPoolSize::default()
                .descriptor_count(n_framebuffers as u32)
                .ty(DescriptorType::SAMPLED_IMAGE),
            vk::DescriptorPoolSize::default()
                .descriptor_count(n_framebuffers as u32)
                .ty(DescriptorType::SAMPLER)];

        let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&descriptor_pool_sizes)
            .max_sets(n_framebuffers as u32);

        let descriptor_pool = unsafe {
            device.device.create_descriptor_pool(&descriptor_pool_create_info, None).unwrap()
        };

        let descriptor_set_layouts = vec![descriptor_set_layout; n_framebuffers];
        let descriptor_set_alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&descriptor_set_layouts);

        let descriptor_sets = unsafe {
            device.device.allocate_descriptor_sets(&descriptor_set_alloc_info).unwrap()
        };

        let mut uniform_buffers = Vec::new();
        for _ in 0..n_framebuffers {
            let mut ubo_buffer = GpuBuffer::<Mat4Ubo>::create_buffer(
                &device,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE
                    | vk::MemoryPropertyFlags::HOST_COHERENT,
                1);

            ubo_buffer.map();
            uniform_buffers.push(ubo_buffer);
        }
        for i in 0..n_framebuffers {
            let descriptor_buffer_info = [vk::DescriptorBufferInfo::default()
                .buffer(uniform_buffers[i].buffer).range(size_of::<Mat4Ubo>() as DeviceSize)];
            let descriptor_image_info = [vk::DescriptorImageInfo::default()
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image_view(texture.image.view)];
            let descriptor_sampler_info = [vk::DescriptorImageInfo::default()
                .sampler(texture.sampler)];
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
                    .descriptor_type(DescriptorType::SAMPLED_IMAGE)
                    .image_info(&descriptor_image_info)
                    .dst_binding(1)
                    .dst_array_element(0)
                    .dst_set(descriptor_sets[i]),
                vk::WriteDescriptorSet::default()
                    .descriptor_count(1)
                    .descriptor_type(DescriptorType::SAMPLER)
                    .image_info(&descriptor_sampler_info)
                    .dst_binding(2)
                    .dst_array_element(0)
                    .dst_set(descriptor_sets[i])
            ];
            unsafe {
                device.device.update_descriptor_sets(&descriptor_write, &[]);
            }
        }
        
        Self {
            device: Arc::downgrade(device),
            index_buffer,
            vertex_buffer,
            uniform_buffers,
            texture,
            descriptor_pool,
            descriptor_sets,
            resolution,
            pipeline_layout,
            current_frame: 0
        }
    }
}

impl RenderDrawlet for TexturedMeshDrawlet {
    type Data = TexturedMeshData;
}

impl VulkanDrawlet for TexturedMeshDrawlet {
    fn draw(self: &Self, command_buffer: CommandBuffer) {
        let device = self.device.upgrade().unwrap();
        unsafe {
            device.device.cmd_bind_vertex_buffers(
                command_buffer,
                0,
                &[self.vertex_buffer.buffer],
                &[0],
            );
            device.device.cmd_bind_index_buffer(
                command_buffer,
                self.index_buffer.buffer,
                0,
                vk::IndexType::UINT32,
            );
            device.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0, self.descriptor_sets[self.current_frame..self.current_frame + 1].as_ref(),
                &[]);
            device.device.cmd_draw_indexed(
                command_buffer,
                6,
                1,
                0,
                0,
                1,
            );
        }
    }
}

impl TexturedMeshDrawlet {
    pub fn set_mvp(self: &mut Self, ubo: Mat4Ubo) {
        let new_ubo = [ubo];
        self.uniform_buffers[self.current_frame].write(&new_ubo);
        self.current_frame = (self.current_frame + 1) % 3;
    }
}

impl Drop for TexturedMeshDrawlet {
    fn drop(&mut self) {
        let device = self.device.upgrade().unwrap();
        unsafe {
            device.device.destroy_descriptor_pool(self.descriptor_pool, None);
        }
    }
}


