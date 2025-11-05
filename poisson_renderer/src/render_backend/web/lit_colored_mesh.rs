use crate::AsAny;
use std::any::Any;
use std::borrow::Cow;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Arc,Weak};
use cgmath::{Matrix4, Vector3};
use egui::IMEPurpose::Normal;
use parking_lot::Mutex;
use wgpu::{ SurfaceConfiguration};
use wgpu::util::DeviceExt;
use poisson_macros::AsAny;
use rj::Own;
use crate::render_backend::{DrawletID, PassID, Mat4Ubo, PipelineID, RenderDrawlet, RenderPipeline};
use crate::render_backend::render_interface::drawlets::{DrawletTrait, PipelineTrait};
use crate::render_backend::render_interface::drawlets::colored_mesh::{ColoredMesh, ColoredMeshData, ColoredMeshDrawletTrait, ColoredVertex};
use crate::render_backend::render_interface::drawlets::lit_colored_mesh::{LitColoredMesh, LitColoredMeshData, LitColoredMeshDrawletTrait, NormalColoredVertex};
use crate::render_backend::web::{WgpuBuffer, Device, WgpuDrawlet, WgpuDrawletDyn, WgpuPipeline, WgpuPipelineDyn, WgpuRenderObject, WgpuRenderPass};
use crate::render_backend::web::gpu_resources::{interface::WgpuUniformResource, gpu_texture::ShaderTexture};
use crate::render_backend::web::gpu_resources::gpu_mat4::GpuMat4;
use crate::render_backend::web::gpu_resources::gpu_texture::Texture;
use crate::render_backend::web::gpu_resources::gpu_vec4::GpuVec4;
use crate::render_backend::web::per_vertex_impl::WgpuPerVertex;

impl WgpuRenderObject for LitColoredMesh {
    type Drawlet = LitColoredMeshDrawlet;
    type Pipeline = LitColoredMeshPipeline;
    type Data = LitColoredMeshData;
}

pub struct LitColoredMeshDrawlet {
    device: Weak<Device>,
    num_indices: u32,
    mvp_buffer: GpuMat4,
    light_buffer: GpuVec4,
    vertex_buffer: rj::Own<WgpuBuffer<NormalColoredVertex>>,
    index_buffer: rj::Own<WgpuBuffer<u32>>
}

impl LitColoredMeshDrawlet {
    fn new(
        device: &Arc<Device>,
        init_data: &LitColoredMeshData
    ) -> Self {
        let uniform_buffer = GpuMat4::from_mat4(&device.device, &init_data.mvp_data);
        let light_dir = GpuVec4::from_vec4(&device.device, &init_data.light_dir);

        let vertex_buffer = init_data.mesh.vertex.buffer.downcast()
            .expect("failed to cast vertex buffer to drawlet buffer type");

        let index_buffer = init_data.mesh.index.buffer.downcast()
            .expect("failed to cast index buffer to drawlet buffer type");

        Self {
            device: Arc::downgrade(device),
            num_indices: init_data.mesh.index.get_count() as u32,
            mvp_buffer: uniform_buffer,
            light_buffer: light_dir,
            vertex_buffer,
            index_buffer
        }
    }
}

impl RenderDrawlet for LitColoredMeshDrawlet {
    type Data = LitColoredMeshData;
}

impl WgpuDrawlet for LitColoredMeshDrawlet {
    fn draw(self: &Self, render_pass: &mut wgpu::RenderPass) {
        render_pass.set_bind_group(0, self.mvp_buffer.get_bind_group(), &[]);
        render_pass.set_bind_group(1, self.light_buffer.get_bind_group(), &[]);

        render_pass.set_vertex_buffer(0, self.vertex_buffer.access().slice());
        render_pass.set_index_buffer(self.index_buffer.access().slice(), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
    }
}

#[derive(AsAny)]
pub struct LitColoredMeshPipeline {
    device: Weak<Device>,
    render_pipeline: wgpu::RenderPipeline,
    drawlets: HashMap<DrawletID, rj::Own<LitColoredMeshDrawlet>>
}

impl WgpuPipelineDyn for LitColoredMeshPipeline {
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

impl RenderPipeline<LitColoredMesh> for LitColoredMeshPipeline {}

impl WgpuPipeline<LitColoredMesh> for LitColoredMeshPipeline {
    fn create_drawlet(self: &mut Self, init_data: LitColoredMeshData) -> rj::Own<LitColoredMeshDrawlet> {
        let id = <Self as RenderPipeline<ColoredMesh>>::get_drawlet_id();
        let new_drawlet = LitColoredMeshDrawlet::new(
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
        let light_dir_bind_group_layout = GpuVec4::create_bind_group_layout(&device.device);

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
                    &light_dir_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let desc = NormalColoredVertex::desc();

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

impl RenderPipeline<ColoredMesh> for LitColoredMeshPipeline {}

impl LitColoredMeshDrawlet {
    pub fn set_mvp(self: &mut Self, ubo: Mat4Ubo) {
        let ubo_slice: &[u8] = unsafe {
            std::slice::from_raw_parts(
                (&ubo as *const Mat4Ubo) as *const u8, size_of::<Mat4Ubo>(),
            )
        };
        self.device.upgrade().as_ref().unwrap().queue.write_buffer(&self.mvp_buffer.buffer, 0, bytemuck::cast_slice(ubo_slice));
    }

    pub fn set_light_dir(self: &mut Self, light_dir: cgmath::Vector3<f32>) {
        let light_dir_slice: &[u8] = unsafe {
            std::slice::from_raw_parts(
                (&light_dir as *const cgmath::Vector3<f32> as *const u8), size_of::<Vector3<f32>>()
            )
        };
        self.device.upgrade().as_ref().unwrap().queue.write_buffer(&self.light_buffer.buffer, 0, bytemuck::cast_slice(light_dir_slice));
    }
}

impl PipelineTrait<LitColoredMesh> for LitColoredMeshPipeline {
    fn create_drawlet(&mut self, init_data: LitColoredMeshData) -> Own<dyn LitColoredMeshDrawletTrait> {
        WgpuPipeline::create_drawlet(self, init_data).upcast()
    }
}

impl DrawletTrait<LitColoredMesh> for LitColoredMeshDrawlet {}

impl LitColoredMeshDrawletTrait for LitColoredMeshDrawlet {
    fn set_mvp(self: &mut Self, mvp: Matrix4<f32>) {
        self.set_mvp(Mat4Ubo {
            data: mvp,
        })
    }

    fn set_light_dir(self: &mut Self, light_dir: Vector3<f32>) {

    }
}

