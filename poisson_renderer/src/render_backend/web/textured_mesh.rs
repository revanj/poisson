use crate::AsAny;
use std::any::Any;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, Weak};
use image::DynamicImage;
use wgpu::{BindGroup, BindGroupLayout, Device, PipelineLayout, Queue, ShaderModule, SurfaceConfiguration};
use wgpu::util::DeviceExt;
use poisson_macros::AsAny;
use crate::render_backend::{DrawletHandle, DrawletID, Mat4Ubo, RenderDrawlet, RenderPipeline, TexturedMesh, TexturedMeshData, Vertex, WgpuDrawlet, WgpuDrawletDyn, WgpuPipeline, WgpuPipelineDyn};
use crate::render_backend::web::{Camera, CameraUniform};
use crate::render_backend::web::texture::Texture;

pub struct TexturedMeshDrawlet {
    queue: Weak<Queue>,
    num_indices: u32,
    texture: Texture,
    texture_bind_group: BindGroup,
    camera_bind_group: BindGroup,
    camera_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
}

impl TexturedMeshDrawlet {
    fn new(
        device: &Device,
        queue: &Arc<Queue>,
        texture_bind_group_layout: &BindGroupLayout,
        camera_bind_group_layout: &BindGroupLayout,
        init_data: &<TexturedMeshDrawlet as RenderDrawlet>::Data
    ) -> Self {
        let camera = Camera {
            // position the camera 1 unit up and 2 units back
            // +z is out of the screen
            eye: (1.0, 2.0, 10.0).into(),
            // have it look at the origin
            target: (0.0, 0.0, 0.0).into(),
            // which way is "up"
            up: cgmath::Vector3::unit_z(),
            aspect: 800f32/600f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        let camera_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let texture = Texture::from_image(device, queue, &init_data.texture_data, Some("TexturedMesh")).expect("failed to create texture");

        let texture_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&texture.sampler),
                    }
                ],
                label: Some("diffuse_bind_group"),
            }
        );

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }
            ],
            label: Some("camera_bind_group"),
        });
        let transformed_vertex: Vec<_> = init_data.vertex_data.iter().map(|x| 
        Vertex {pos: x.pos, tex_coord: [x.tex_coord[0], 1f32 - x.tex_coord[1]]}).collect();
        
        let vertex_data: &[u8] = unsafe {
            std::slice::from_raw_parts(
                transformed_vertex.as_ptr() as *const u8,
                transformed_vertex.len() * std::mem::size_of::<Vertex>()
            )
        };
        let index_data: &[u8] = unsafe {
            std::slice::from_raw_parts(
                init_data.index_data.as_ptr() as *const u8,
                init_data.index_data.len() * std::mem::size_of::<u32>()
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
            num_indices: init_data.index_data.len() as u32,
            texture,
            camera_buffer,
            texture_bind_group,
            camera_bind_group,
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
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.set_bind_group(1, &self.texture_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
    }
}

#[derive(AsAny)]
pub struct TexturedMeshPipeline {
    device: Weak<Device>,
    queue: Weak<wgpu::Queue>,
    camera_bind_group_layout: BindGroupLayout,
    texture_bind_group_layout: BindGroupLayout,
    shader_module: ShaderModule,
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
    fn instantiate_drawlet(self: &mut Self, init_data: TexturedMeshData) -> DrawletHandle<TexturedMesh> {
        let id = <Self as RenderPipeline<TexturedMesh>>::get_drawlet_id();
        let new_drawlet = TexturedMeshDrawlet::new(
            &self.device.upgrade().unwrap(),
            &self.queue.upgrade().unwrap(),
            &self.texture_bind_group_layout,
            &self.camera_bind_group_layout , &init_data);

        self.drawlets.insert(id, new_drawlet);

        DrawletHandle {
            id,
            _drawlet_ty: Default::default()
        }
    }

    fn get_drawlet_mut(self: &mut Self, drawlet_handle: &DrawletHandle<TexturedMesh>) -> &'_ mut TexturedMeshDrawlet {
        self.drawlets.get_mut(&drawlet_handle.id).unwrap()
    }
    fn new(device: &Arc<Device>, queue: &Arc<Queue>, shader_path: &str, surface_config: &SurfaceConfiguration) -> Self
        where Self: Sized
    {
        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("camera_bind_group_layout"),
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let compiler = slang_refl::Compiler::new_wgsl_compiler();
        let linked_program = compiler.linked_program_from_file(shader_path);

        let compiled_triangle_shader = linked_program.get_u8();
        let wgsl_str = str::from_utf8(compiled_triangle_shader).unwrap();
        
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

        let desc = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                }
            ]
        };

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"), // 1.
                buffers: &[desc], // 2.
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },

            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // 1.
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, // 2.
                cull_mode: None,
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None, // 1.
            multisample: wgpu::MultisampleState {
                count: 1, // 2.
                mask: !0, // 3.
                alpha_to_coverage_enabled: false, // 4.
            },
            multiview: None, // 5.
            cache: None, // 6.
        });

        let drawlets = HashMap::new();

        Self {
            device: Arc::downgrade(device),
            queue: Arc::downgrade(queue),
            camera_bind_group_layout,
            texture_bind_group_layout,
            shader_module: shader,
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
                (&ubo as *const Mat4Ubo) as *const u8,
                ::core::mem::size_of::<Mat4Ubo>(),
            )
        };
        self.queue.upgrade().as_ref().unwrap().write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(ubo_slice));
    }
}