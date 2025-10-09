pub mod textured_mesh;
mod gpu_resources;
mod per_vertex_impl;
mod colored_mesh;

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::marker::PhantomData;
use std::sync::{Arc, Weak};
use parking_lot::Mutex;
use winit::window::Window;
use crate::render_backend::{DrawletHandle, PipelineHandle, PipelineID, RenderBackend, RenderDrawlet, LayerHandle, LayerID, RenderPipeline, ViewHandle, ViewID};
use wgpu;
use winit::dpi::PhysicalSize;
use bytemuck;
use cfg_if::cfg_if;
use cgmath::Matrix4;
use wgpu::util::DeviceExt;
use image;
use image::EncodableLayout;
use wgpu::{BindGroup, BindGroupLayout, CommandEncoder, SurfaceConfiguration, TextureFormat, TextureView};
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowExtWeb;
use crate::{AsAny, PoissonGame};
use crate::input::Input;
use crate::render_backend::render_interface::RenderObject;
use crate::render_backend::web::textured_mesh::TexturedMeshDrawlet;

pub trait WgpuRenderObject: RenderObject + Sized {
    type Drawlet: WgpuDrawlet;
    type Pipeline: WgpuPipeline<Self> + WgpuPipelineDyn + 'static;
    type Data;
}



pub trait 
WgpuPipeline<RenObjType: WgpuRenderObject>: RenderPipeline<RenObjType> + WgpuPipelineDyn {
    fn instantiate_drawlet(
        self: &mut Self,
        layer_id: LayerID,
        pipeline_id: PipelineID,
        init_data: <<RenObjType as WgpuRenderObject>::Drawlet as RenderDrawlet>::Data
    ) -> DrawletHandle<RenObjType>;

    fn get_drawlet_mut(self: &mut Self, drawlet_handle: &DrawletHandle<RenObjType>) -> &'_ mut RenObjType::Drawlet ;
    fn new(
        device: &Arc<wgpu::Device>,
        queue: &Arc<wgpu::Queue>,
        shader_u8: &[u8],
        surface_config: &SurfaceConfiguration
    ) -> Self where Self: Sized;
}

pub trait WgpuPipelineDyn: AsAny {
    fn get_pipeline(self: &Self) -> &wgpu::RenderPipeline;
    fn get_instances(self: &Self) -> Box<dyn Iterator<Item=&dyn WgpuDrawletDyn> + '_>;
    fn get_instances_mut(self: &mut Self) -> Box<dyn Iterator<Item=&mut dyn WgpuDrawletDyn> + '_>;
}

pub trait WgpuDrawlet: RenderDrawlet {
    fn draw(self: &Self, render_pass: &mut wgpu::RenderPass);
}
pub trait WgpuDrawletDyn {
    fn draw(self: &Self, render_pass: &mut wgpu::RenderPass);
}
impl<T> WgpuDrawletDyn for T where T: WgpuDrawlet {
    fn draw(self: &Self, render_pass: &mut wgpu::RenderPass) {
        self.draw(render_pass);
    }
}

struct SharedRenderResource {
    camera_bind_group_layout: BindGroupLayout,
    camera_bind_group: BindGroup
}

struct CameraController {
    speed: f32,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
    last_update_duration: instant::Instant
}

impl CameraController {
    fn new(speed: f32) -> Self {
        Self {
            speed,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
            last_update_duration: instant::Instant::now()
        }
    }

    fn process_events(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event:
                KeyEvent {
                    state,
                    physical_key: PhysicalKey::Code(keycode),
                    ..
                },
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                match keycode {KeyCode::KeyW | KeyCode::ArrowUp => {
                    self.is_forward_pressed = is_pressed;
                    true
                }
                    KeyCode::KeyA | KeyCode::ArrowLeft => {
                        self.is_left_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyS | KeyCode::ArrowDown => {
                        self.is_backward_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyD | KeyCode::ArrowRight => {
                        self.is_right_pressed = is_pressed;
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn update_camera(&mut self, camera: &mut Camera) {
        use cgmath::InnerSpace;
        let forward = (camera.target - camera.eye);
        let forward_norm = forward.normalize();
        let forward_mag = forward.magnitude();

        let now = instant::Instant::now();
        let dt = now - self.last_update_duration;
        self.last_update_duration = now;

        let dt = dt.as_secs_f32();


        // Prevents glitching when the camera gets too close to the
        // center of the scene.
        if self.is_forward_pressed && forward_mag > self.speed {
            camera.eye += forward_norm * self.speed * dt;
        }
        if self.is_backward_pressed {
            camera.eye -= forward_norm * self.speed * dt;
        }

        let right = forward_norm.cross(camera.up);

        // Redo radius calc in case the forward/backward is pressed.
        let forward = camera.target - camera.eye;
        let forward_mag = forward.magnitude();

        log::info!("dt is {}", dt);

        if self.is_right_pressed {
            // Rescale the distance between the target and the eye so
            // that it doesn't change. The eye, therefore, still
            // lies on the circle made by the target and eye.
            camera.eye = camera.target - (forward + right * self.speed * dt).normalize() * forward_mag;
        }
        if self.is_left_pressed {
            camera.eye = camera.target - (forward - right * self.speed * dt).normalize() * forward_mag;
        }
    }
}


#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into();
    }
}

struct Camera {
    eye: cgmath::Point3<f32>,
    target: cgmath::Point3<f32>,
    up: cgmath::Vector3<f32>,
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::from_cols(
    cgmath::Vector4::new(1.0, 0.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 1.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 1.0),
);


impl Camera {
    fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
        return OPENGL_TO_WGPU_MATRIX * proj * view;
    }
}

pub struct WgpuRenderPass {
    pipelines: HashMap<PipelineID, Box<dyn WgpuPipelineDyn>>
}

impl WgpuRenderPass {
    fn new() -> Self {
        Self {
            pipelines: HashMap::new()
        }
    }
    fn render(self: &Self, encoder: &mut CommandEncoder, target_view: &TextureView) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Discard,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });
        for (_, pipeline) in &self.pipelines {
            {
                render_pass.set_pipeline(pipeline.get_pipeline());
            }
            for drawlet in pipeline.get_instances() {
                drawlet.draw(&mut render_pass);
            }
        }
    }
}

pub struct WgpuRenderBackend {
    surface: wgpu::Surface<'static>,
    _adapter: wgpu::Adapter,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    size_changed: bool,
    max_size: winit::dpi::PhysicalSize<u32>,
    //pipelines: HashMap<PipelineID, Box<dyn WgpuPipelineDyn>>,
    render_passes: HashMap<LayerID, WgpuRenderPass>,
}


impl RenderBackend for WgpuRenderBackend {
    const PERSPECTIVE_ALIGNMENT: [f32; 3] = [1f32, 1f32, -1f32];

    fn init(backend_clone: Arc<Mutex<Option<Self>>>, window: Arc<dyn Window>) where Self: Sized
    {
        cfg_if::cfg_if! {
            if #[cfg(target_arch="wasm32")] {
                log::info!("running wasm32 backend creation");
                wasm_bindgen_futures::spawn_local(async move {
                    let new_backend = WgpuRenderBackend::new(&window).await;
                    let mut locked_backend = backend_clone.lock();
                    *locked_backend = Some(new_backend);
                });
            } else {
                let render_backend = pollster::block_on(WgpuRenderBackend::new(&window));
                backend_clone.lock().replace(render_backend);
            }
        }
    }

    fn render(self: &mut Self) {
        self.resize_surface_if_needed();

        let output = self.surface.get_current_texture().unwrap();
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            for render_pass in self.render_passes.values() {
                render_pass.render(&mut encoder, &view);
            }

        }

        self.queue.submit(Some(encoder.finish()));
        output.present();
    }

    fn process_event(self: &mut Self, event: &WindowEvent) {
    }

    fn resize(self: &mut Self, width: u32, height: u32) {
        let new_size = PhysicalSize::<u32> {
            width,
            height
        };

        if new_size == self.size {
            return;
        }

        self.size = new_size;
        self.size_changed = true;
    }

    fn get_default_view(self: &Self) -> ViewHandle {
        ViewHandle {
            id: ViewID(0)
        }
    }

    fn create_view(self: &mut Self, view_proj: Matrix4<f32>) -> ViewHandle {
        todo!()
    }

    fn set_view(self: &mut Self, view_handle: ViewHandle, view_proj: Matrix4<f32>) {
        todo!()
    }
}

impl WgpuRenderBackend {
    fn resize_surface_if_needed(&mut self) {
        if self.size_changed {
            self.config.width = self.size.width.min(self.max_size.width);
            self.config.height = self.size.height.min(self.max_size.height);
            self.surface.configure(&self.device, &self.config);
            log::info!("width and height is, {}, {}", self.config.width, self.config.height);
            self.size_changed = false;
        }
    }

    pub async fn new(window: &Arc<dyn Window>) -> Self {
        #[cfg(any(target_arch = "wasm32"))]
        {
            let canvas = window.canvas().unwrap();

            web_sys::window()
                .and_then(|win| win.document())
                .map(|doc| {
                    let _ = canvas.set_attribute("id", "winit-canvas");
                    match doc.get_element_by_id("wgpu-app-container") {
                        Some(dst) => {
                            let _ = dst.append_child(canvas.as_ref());
                        }
                        None => {
                            let container = doc.create_element("div").unwrap();
                            let _ = container.set_attribute("id", "wgpu-app-container");
                            let _ = container.append_child(canvas.as_ref());

                            doc.body().map(|body| body.append_child(container.as_ref()));
                        }
                    };
                })
                .expect("无法将 canvas 添加到当前网页中");

            // 确保画布可以获得焦点
            // https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/tabindex
            canvas.set_tab_index(0);

            // 设置画布获得焦点时不显示高亮轮廓
            let style = canvas.style();
            style.set_property("outline", "none").unwrap();
            canvas.focus().expect("画布无法获取焦点");
        }

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::GL,
            ..Default::default()
        });
        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits:
                    if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults() }
                    else { wgpu::Limits::default() },
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .unwrap();

        let mut size = window.surface_size();
        size.width = size.width.max(800);
        size.height = size.height.max(600);

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb()) // wgpu 0.20+ helper
            .unwrap_or(wgpu::TextureFormat::Rgba8UnormSrgb);

        let mut config = surface
            .get_default_config(&adapter, size.width, size.height)
            .unwrap();
        config.format = TextureFormat::Rgba8Unorm;

        surface.configure(&device, &config);

        Self {
            surface,
            _adapter: adapter,
            device: Arc::new(device),
            queue: Arc::new(queue),
            config,
            size,
            size_changed: false,
            max_size: PhysicalSize {width: 800, height: 600},
            render_passes: HashMap::new()
        }
    }
}

impl CreateDrawletWgpu for WgpuRenderBackend
{
    fn create_render_pass(self: &mut Self) -> LayerHandle {
        let id = Self::get_render_pass_id();
        self.render_passes.insert(id.clone(), WgpuRenderPass {
            pipelines: Default::default(),
        });

        LayerHandle { id }
    }

    fn create_pipeline<RenObjType: WgpuRenderObject>(self: &mut Self, render_pass_handle: &LayerHandle, shader_path: &str, shader_text: &str) -> PipelineHandle<RenObjType> {
        let owned_str = shader_path.to_owned();
        let wgsl_path = owned_str.clone() + ".wgsl";
        #[cfg(not(target_arch="wasm32"))]
        {
            let compiler = slang_refl::Compiler::new_wgsl_compiler();
            let slang_path = owned_str + ".slang";
            let linked_program = compiler.linked_program_from_file(slang_path.as_str());
            let compiled_shader = linked_program.get_u8();
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(wgsl_path.clone()).unwrap();
            
            file.write_all(compiled_shader).unwrap()
        }
        let wgsl_code;
        
        cfg_if! {
            if #[cfg(not(target_arch="wasm32"))] {
                wgsl_code = fs::read(wgsl_path).unwrap();
            } else {
                wgsl_code = shader_text.to_owned();
            }
        }
        
        let pipeline = RenObjType::Pipeline::new(
            &self.device, &self.queue, wgsl_code.as_bytes(), &self.config);

        let pipeline_id: PipelineID = Self::get_pipeline_id();

        let ret = PipelineHandle::<RenObjType> {
            id: pipeline_id,
            layer_id: render_pass_handle.id,
            _pipeline_ty: PhantomData::default(),
        };


        self.render_passes.get_mut(&render_pass_handle.id).unwrap().pipelines.insert(pipeline_id.clone(), Box::new(pipeline));

        ret
    }

    fn create_drawlet<RenObjType: WgpuRenderObject>(self: &mut Self, pipeline_handle: &PipelineHandle<RenObjType>, init_data: <<RenObjType as WgpuRenderObject>::Drawlet as RenderDrawlet>::Data) -> DrawletHandle<RenObjType>
    {
        let pipeline= self.render_passes.get_mut(&pipeline_handle.layer_id).unwrap().pipelines.get_mut(&pipeline_handle.id).unwrap();
        let pipeline_any = pipeline.as_any_mut();
        let pipeline_concrete = pipeline_any.downcast_mut::<RenObjType::Pipeline>().unwrap();

        pipeline_concrete.instantiate_drawlet(pipeline_handle.layer_id, pipeline_handle.id, init_data)
    }

    fn get_drawlet_mut<RenObjType: WgpuRenderObject>(self: &mut Self, drawlet_handle: &DrawletHandle<RenObjType>) -> &'_ mut RenObjType::Drawlet {
        let pipeline = self.render_passes.get_mut(&drawlet_handle.layer_id).unwrap().pipelines.get_mut(&drawlet_handle.pipeline_id).unwrap();
        let pipeline_any = pipeline.as_any_mut();
        let pipeline_concrete = pipeline_any.downcast_mut::<RenObjType::Pipeline>().unwrap();

        pipeline_concrete.get_drawlet_mut(&drawlet_handle)
    }
}

pub trait CreateDrawletWgpu
{
    fn create_render_pass(
        self: &mut Self
    ) -> LayerHandle;

    fn create_pipeline<RenObjType: WgpuRenderObject>(
        self: &mut Self,
        render_pass_handle: &LayerHandle,
        shader_path: &str,
        shader_text: &str,
    ) -> PipelineHandle<RenObjType>;

    fn create_drawlet<RenObjType: WgpuRenderObject>(
        self: &mut Self,
        pipeline: &PipelineHandle<RenObjType>,
        init_data: <RenObjType::Drawlet as RenderDrawlet>::Data,
    ) -> DrawletHandle<RenObjType>;

    fn get_drawlet_mut<RenObjType: WgpuRenderObject>(
        self: &mut Self,
        drawlet_handle: &DrawletHandle<RenObjType>
    ) -> &'_ mut RenObjType::Drawlet;
}

