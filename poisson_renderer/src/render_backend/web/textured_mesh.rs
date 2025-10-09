use crate::AsAny;
use std::any::Any;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, Weak};
use wgpu::{BindGroupLayout, Device, Queue, ShaderModule, SurfaceConfiguration};
use wgpu::util::DeviceExt;
use poisson_macros::AsAny;
use crate::render_backend::{DrawletHandle, DrawletID, LayerID, Mat4Ubo, PipelineID, RenderDrawlet, RenderPipeline};
use crate::render_backend::render_interface::{TexVertex, TexturedMesh, TexturedMeshData};
use crate::render_backend::web::{WgpuDrawlet, WgpuDrawletDyn, WgpuPipeline, WgpuPipelineDyn, WgpuRenderObject};
use crate::render_backend::web::gpu_resources::{interface::WgpuUniformResource, gpu_texture::ShaderTexture};
use crate::render_backend::web::gpu_resources::gpu_mat4::GpuMat4;
use crate::render_backend::web::gpu_resources::gpu_texture::Texture;
use crate::render_backend::web::per_vertex_impl::WgpuPerVertex;

impl WgpuRenderObject for TexturedMesh {
    type Drawlet = TexturedMeshDrawlet;
    type Pipeline = TexturedMeshPipeline;
    type Data = TexturedMeshData;
}

pub struct TexturedMeshDrawlet {
    queue: Weak<Queue>,
    num_indices: u32,
    gpu_texture: ShaderTexture,
    mvp_buffer: GpuMat4,
    index_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
}

impl TexturedMeshDrawlet {
    fn new(
        device: &Device,
        queue: &Arc<Queue>,
        init_data: &<TexturedMeshDrawlet as RenderDrawlet>::Data
    ) -> Self {
        let uniform_buffer = GpuMat4::from_mat4(device, &init_data.mvp_data);

        let texture = ShaderTexture::from_image(
            device, 
            queue,
            &init_data.texture_data, 
            Some("TexturedMesh")
        ).expect("failed to create texture");

        let vertex_data: &[u8] = unsafe {
            std::slice::from_raw_parts(
                init_data.mesh.vertex_data.as_ptr() as *const u8,
                init_data.mesh.vertex_data.len() * size_of::<TexVertex>()
            )
        };
        let index_data: &[u8] = unsafe {
            std::slice::from_raw_parts(
                init_data.mesh.index_data.as_ptr() as *const u8,
                init_data.mesh.index_data.len() * size_of::<u32>()
            )
        };
        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: vertex_data,
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: index_data,
                usage: wgpu::BufferUsages::INDEX,
            }
        );

        Self {
            queue: Arc::downgrade(queue),
            num_indices: init_data.mesh.index_data.len() as u32,
            gpu_texture: texture,
            mvp_buffer: uniform_buffer,
            vertex_buffer,
            index_buffer
        }
    }
}

impl RenderDrawlet for TexturedMeshDrawlet {
    type Data = TexturedMeshData;
}

impl WgpuDrawlet for TexturedMeshDrawlet {
    fn draw(self: &Self, render_pass: &mut wgpu::RenderPass) {
        render_pass.set_bind_group(0, self.mvp_buffer.get_bind_group(), &[]);
        render_pass.set_bind_group(1, self.gpu_texture.get_bind_group(), &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
    }
}

#[derive(AsAny)]
pub struct TexturedMeshPipeline {
    device: Weak<Device>,
    queue: Weak<wgpu::Queue>,
    render_pipeline: wgpu::RenderPipeline,
    drawlets: HashMap<DrawletID, TexturedMeshDrawlet>
}

impl WgpuPipelineDyn for TexturedMeshPipeline {
    fn get_pipeline(self: &Self) -> &wgpu::RenderPipeline {
        &self.render_pipeline
    }
    fn get_instances(self: &Self) -> Box<dyn Iterator<Item=&dyn WgpuDrawletDyn> + '_> {
        Box::new(self.drawlets.iter().map(|(_, x)| x as &dyn WgpuDrawletDyn))
    }
    fn get_instances_mut(self: &mut Self) -> Box<dyn Iterator<Item=&mut dyn WgpuDrawletDyn> + '_> {
        Box::new(self.drawlets.iter_mut().map(|(_, x)| x as &mut dyn WgpuDrawletDyn))
    }
}

impl WgpuPipeline<TexturedMesh> for TexturedMeshPipeline {
    fn instantiate_drawlet(self: &mut Self, layer_id: LayerID, pipeline_id: PipelineID, init_data: TexturedMeshData) -> DrawletHandle<TexturedMesh> {
        let id = <Self as RenderPipeline<TexturedMesh>>::get_drawlet_id();
        let new_drawlet = TexturedMeshDrawlet::new(
            &self.device.upgrade().unwrap(),
            &self.queue.upgrade().unwrap(),
            &init_data);

        self.drawlets.insert(id, new_drawlet);

        DrawletHandle {
            id,
            pipeline_id,
            layer_id,
            _drawlet_ty: Default::default()
        }
    }

    fn get_drawlet_mut(self: &mut Self, drawlet_handle: &DrawletHandle<TexturedMesh>) -> &'_ mut TexturedMeshDrawlet {
        self.drawlets.get_mut(&drawlet_handle.id).unwrap()
    }
    fn new(device: &Arc<Device>, queue: &Arc<Queue>, shader_u8: &[u8], surface_config: &SurfaceConfiguration) -> Self
        where Self: Sized
    {

        let camera_bind_group_layout = GpuMat4::create_bind_group_layout(device);

        let texture_bind_group_layout = ShaderTexture::create_bind_group_layout(device);

        let wgsl_str = str::from_utf8(shader_u8).unwrap();

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::from(wgsl_str)),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &camera_bind_group_layout,
                    &texture_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let desc = TexVertex::desc();

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                buffers: &[desc],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },

            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(
                wgpu::DepthStencilState {
                    format: Texture::DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let drawlets = HashMap::new();

        Self {
            device: Arc::downgrade(device),
            queue: Arc::downgrade(queue),
            render_pipeline,
            drawlets
        }
    }
}

impl RenderPipeline<TexturedMesh> for TexturedMeshPipeline {}

impl TexturedMeshDrawlet {
    pub fn set_mvp(self: &mut Self, ubo: Mat4Ubo) {
        let ubo_slice: &[u8] = unsafe {
            std::slice::from_raw_parts(
                (&ubo as *const Mat4Ubo) as *const u8, size_of::<Mat4Ubo>(),
            )
        };
        self.queue.upgrade().as_ref().unwrap().write_buffer(&self.mvp_buffer.buffer, 0, bytemuck::cast_slice(ubo_slice));
    }
}