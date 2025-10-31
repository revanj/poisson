use crate::AsAny;
use std::any::Any;
use std::borrow::Cow;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Arc,Weak};
use parking_lot::Mutex;
use wgpu::{ SurfaceConfiguration};
use wgpu::util::DeviceExt;
use poisson_macros::AsAny;
use crate::render_backend::{DrawletID, LayerID, Mat4Ubo, PipelineID, RenderDrawlet, RenderPipeline};
use crate::render_backend::render_interface::{ColoredMesh, ColoredMeshData, ColoredVertex, Mesh};
use crate::render_backend::web::{WgpuBuffer, Device, WgpuDrawlet, WgpuDrawletDyn, WgpuPipeline, WgpuPipelineDyn, WgpuRenderObject};
use crate::render_backend::web::gpu_resources::{interface::WgpuUniformResource, gpu_texture::ShaderTexture};
use crate::render_backend::web::gpu_resources::gpu_mat4::GpuMat4;
use crate::render_backend::web::gpu_resources::gpu_texture::Texture;
use crate::render_backend::web::per_vertex_impl::WgpuPerVertex;

impl WgpuRenderObject for ColoredMesh {
    type Drawlet = ColoredMeshDrawlet;
    type Pipeline = ColoredMeshPipeline;
    type Data = ColoredMeshData;
}

pub struct ColoredMeshDrawlet {
    device: Weak<Device>,
    num_indices: u32,
    mvp_buffer: GpuMat4,
    vertex_buffer: rj::Own<WgpuBuffer<ColoredVertex>>,
    index_buffer: rj::Own<WgpuBuffer<u32>>
}

impl ColoredMeshDrawlet {
    fn new(
        device: &Arc<Device>,
        init_data: &ColoredMeshData
    ) -> Self {
        let uniform_buffer = GpuMat4::from_mat4(&device.device, &init_data.mvp_data);

        let vertex_buffer = init_data.mesh.vertex.buffer.downcast()
            .expect("failed to cast vertex buffer to drawlet buffer type");

        let index_buffer = init_data.mesh.index.buffer.downcast()
            .expect("failed to cast index buffer to drawlet buffer type");


        Self {
            device: Arc::downgrade(device),
            num_indices: init_data.mesh.index.get_count() as u32,
            mvp_buffer: uniform_buffer,
            vertex_buffer,
            index_buffer
        }
    }
}

impl RenderDrawlet for ColoredMeshDrawlet {
    type Data = ColoredMeshData;
}

impl WgpuDrawlet for ColoredMeshDrawlet {
    fn draw(self: &Self, render_pass: &mut wgpu::RenderPass) {
        render_pass.set_bind_group(0, self.mvp_buffer.get_bind_group(), &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.access().slice());
        render_pass.set_index_buffer(self.index_buffer.access().slice(), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
    }
}

#[derive(AsAny)]
pub struct ColoredMeshPipeline {
    device: Weak<Device>,
    render_pipeline: wgpu::RenderPipeline,
    drawlets: HashMap<DrawletID, rj::Own<ColoredMeshDrawlet>>
}

impl WgpuPipelineDyn for ColoredMeshPipeline {
    fn get_pipeline(self: &Self) -> &wgpu::RenderPipeline {
        &self.render_pipeline
    }
    fn get_instances(self: &Self) -> Box<dyn Iterator<Item=rj::Own<dyn WgpuDrawletDyn>> + '_> {
        Box::new(self.drawlets.iter().map(
            |(_, x)|
                rj::Own::<dyn WgpuDrawletDyn>::from_inner(x.clone().into_inner())
        ))
    }
}

impl WgpuPipeline<ColoredMesh> for ColoredMeshPipeline {
    fn create_drawlet(self: &mut Self, init_data: ColoredMeshData) -> rj::Own<ColoredMeshDrawlet> {
        let id = <Self as RenderPipeline<ColoredMesh>>::get_drawlet_id();
        let new_drawlet = ColoredMeshDrawlet::new(
            &self.device.upgrade().unwrap(),
            &init_data);

        let own = rj::Own::new(new_drawlet);

        self.drawlets.insert(id, own.clone());

        own
    }
    
    fn new(device: &Arc<Device>, shader_u8: &[u8], surface_config: &SurfaceConfiguration) -> Self
    where Self: Sized
    {
        let camera_bind_group_layout = GpuMat4::create_bind_group_layout(&device.device);


        let wgsl_str = str::from_utf8(shader_u8).unwrap();

        let shader = device.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::from(wgsl_str)),
        });

        let render_pipeline_layout =
            device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &camera_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let desc = ColoredVertex::desc();

        let render_pipeline = device.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
            render_pipeline,
            drawlets
        }
    }
}

impl RenderPipeline<ColoredMesh> for ColoredMeshPipeline {}

impl ColoredMeshDrawlet {
    pub fn set_mvp(self: &mut Self, ubo: Mat4Ubo) {
        let ubo_slice: &[u8] = unsafe {
            std::slice::from_raw_parts(
                (&ubo as *const Mat4Ubo) as *const u8, size_of::<Mat4Ubo>(),
            )
        };
        self.device.upgrade().as_ref().unwrap().queue.write_buffer(&self.mvp_buffer.buffer, 0, bytemuck::cast_slice(ubo_slice));
    }
}