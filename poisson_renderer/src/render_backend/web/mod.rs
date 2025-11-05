pub mod textured_mesh;
mod gpu_resources;
mod per_vertex_impl;
pub mod colored_mesh;

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::marker::PhantomData;
use std::sync::{Arc};
use winit::window::Window;
use crate::render_backend::{PipelineID, RenderBackend, RenderDrawlet, PassID, RenderPipeline};
use wgpu;
use winit::dpi::PhysicalSize;
use cfg_if::cfg_if;
use egui_wgpu::ScreenDescriptor;
use image::EncodableLayout;
use wgpu::util::DeviceExt;
use parking_lot::Mutex;
use wgpu::{BufferSlice, CommandEncoder, SurfaceConfiguration, TextureFormat, TextureView};
use winit::event::{WindowEvent};

#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowExtWebSys;

use crate::{AsAny};
use crate::render_backend::render_interface::{RenderObject};
use crate::render_backend::web::gpu_resources::gpu_texture::Texture;

pub trait EguiUiShow {
    fn show(&mut self, ctx: &egui::Context);
}

pub trait WgpuRenderObject: RenderObject + Sized {
    type Drawlet: WgpuDrawlet;
    type Pipeline: WgpuPipeline<Self> + WgpuPipelineDyn + 'static;
    type Data;
}



pub trait 
WgpuPipeline<RenObjType: WgpuRenderObject>: RenderPipeline<RenObjType> + WgpuPipelineDyn {
    fn create_drawlet(
        self: &mut Self,
        init_data: <<RenObjType as WgpuRenderObject>::Drawlet as RenderDrawlet>::Data
    ) -> rj::Own<<RenObjType as WgpuRenderObject>::Drawlet>;

    fn new(
        device: &Arc<Device>,
        shader_u8: &[u8],
        surface_config: &SurfaceConfiguration
    ) -> Self where Self: Sized;
}

pub trait WgpuPipelineDyn: AsAny {
    fn get_pipeline(self: &Self) -> &wgpu::RenderPipeline;
    fn get_instances(self: &Self) -> Box<dyn Iterator<Item=rj::Own<dyn WgpuDrawletDyn>> + '_>;
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


pub struct WgpuRenderPass {
    device: std::sync::Weak<Device>,
    surface_config: SurfaceConfiguration,
    depth_stencil: Texture,
    pipelines: HashMap<PipelineID, rj::Own<dyn WgpuPipelineDyn>>
}

impl PassTrait for WgpuRenderPass {
    fn create_textured_mesh_pipeline(&mut self, shader_path: &str, shader_text: &str) -> (PipelineID, Own<(dyn PipelineTrait<TexturedMesh> + 'static)>) {
        let (id, pipe) = self.create_pipeline::<TexturedMesh>(shader_path, shader_text);

        (id, pipe.upcast())
    }

    fn create_colored_mesh_pipeline(&mut self, shader_path: &str, shader_text: &str) -> (PipelineID, Own<(dyn PipelineTrait<ColoredMesh> + 'static)>) {
        let (id, pipe) = self.create_pipeline::<ColoredMesh>(shader_path, shader_text);

        (id, pipe.upcast())
    }
}

impl WgpuRenderPass {
    fn new(device: &Arc<Device>, surface_configuration: &SurfaceConfiguration) -> Self {
        Self {
            device: Arc::downgrade(device),
            surface_config: surface_configuration.clone(),
            depth_stencil:
                Texture::create_depth_texture(
                    &device.device,
                    surface_configuration,
                    "depth stencil texture"),
            pipelines: HashMap::new()
        }
    }

    pub fn create_pipeline<RenObjType: WgpuRenderObject>(self: &mut Self, shader_path: &str, shader_text: &str) -> (PipelineID, rj::Own<<RenObjType as WgpuRenderObject>::Pipeline>) {
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

        let inner = Arc::new(Mutex::new(RenObjType::Pipeline::new(
            &self.device.upgrade().as_ref().unwrap(), wgsl_code.as_bytes(), &self.surface_config
        )));

        let pipeline: rj::Own<dyn WgpuPipelineDyn + 'static> =
            rj::Own::<dyn WgpuPipelineDyn>::from_inner(inner.clone());

        let pipeline_id: PipelineID = Self::get_pipeline_id();

        self.pipelines.insert(pipeline_id.clone(), pipeline.clone());

        (pipeline_id.clone(), rj::Own::from_inner(inner))
    }

    pub fn get_pipeline_id() -> PipelineID {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER:AtomicUsize = AtomicUsize::new(1);
        PipelineID(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    fn render(self: &Self, encoder: &mut CommandEncoder, target_view: &TextureView) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(
                wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 1.0,
                            g: 1.0,
                            b: 1.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_stencil.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });

        for (_, pipeline) in &self.pipelines {
            { render_pass.set_pipeline(pipeline.access().get_pipeline()); }
            for drawlet in pipeline.access().get_instances() {
                drawlet.access().draw(&mut render_pass);
            }
        }
    }
}

pub struct Device {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue
}

#[derive()]
pub struct WgpuRenderBackend {
    surface: wgpu::Surface<'static>,
    _adapter: wgpu::Adapter,
    device: Arc<Device>,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    size_changed: bool,
    max_size: winit::dpi::PhysicalSize<u32>,
    render_passes: HashMap<PassID, rj::Own<WgpuRenderPass>>,
    egui_renderer: EguiRenderer,
}


impl RenderBackend for WgpuRenderBackend {
    const PERSPECTIVE_ALIGNMENT: [f32; 3] = [1f32, 1f32, -1f32];

    fn init(backend_clone: Arc<Mutex<Option<Self>>>, window: Arc<Window>) where Self: Sized
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

    fn render(self: &mut Self, window: &Arc<Window>, egui_show_obj: &mut dyn EguiUiShow) {
        self.resize_surface_if_needed(window);

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: window.scale_factor() as f32,
        };

        let output = self.surface.get_current_texture().unwrap();
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            for render_pass in self.render_passes.values() {
                render_pass.access().render(&mut encoder, &view);
            }

            self.egui_renderer.begin_frame(window);

            egui_show_obj.show(self.egui_renderer.context());

            self.egui_renderer.end_frame_and_draw(
                &self.device.device,
                &self.device.queue,
                &mut encoder,
                window,
                &view,
                screen_descriptor,
            );
        }

        self.device.queue.submit(Some(encoder.finish()));
        output.present();
    }

    fn process_event(self: &mut Self, window: &Window, event: &WindowEvent) {
        self.egui_renderer
            .handle_input(window, &event);
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

    fn create_index_buffer(self: &Self, data: &[u32]) -> GpuBufferHandle<u32> {
        let index_data: &[u8] = unsafe {
            std::slice::from_raw_parts(
                data.as_ptr() as *const u8, data.len() * size_of::<u32>()
            )
        };

        let index_buffer = self.device.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: index_data,
                usage: wgpu::BufferUsages::INDEX,
            }
        );

        let buffer_own = rj::Own::new(
            WgpuBuffer::<u32> {
                buffer: index_buffer,
                size: data.len(),
                _phantom_data: PhantomData::default()
            });
        

        GpuBufferHandle::from_own(buffer_own.upcast())
        
    }

    fn create_vertex_buffer<T:Sized + 'static>(self: &Self, data: &[T]) -> GpuBufferHandle<T> {
        let vertex_data: &[u8] = unsafe {
            std::slice::from_raw_parts(
                data.as_ptr() as *const u8, data.len() * size_of::<T>()
            )
        };

        let vertex_buffer = self.device.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: vertex_data,
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        let buffer_own = rj::Own::new(
            WgpuBuffer::<T> {
                buffer: vertex_buffer,
                size: data.len(),
                _phantom_data: PhantomData::default()
            }
        );

        GpuBufferHandle::from_own(buffer_own.upcast())
    }

    fn get_width(self: &Self) -> u32 {
        self.size.width
    }

    fn get_height(self: &Self) -> u32 {
        self.size.height
    }

    fn get_egui_renderer(self: &Self) -> EguiRenderer {
        todo!()
    }
}

use wasm_bindgen::JsCast;
use poisson_macros::AsAny;
use rj::Own;
use crate::egui::EguiRenderer;
use crate::render_backend::render_interface::drawlets::{PassHandle, PassTrait, PipelineTrait};
use crate::render_backend::render_interface::drawlets::colored_mesh::ColoredMesh;
use crate::render_backend::render_interface::drawlets::textured_mesh::TexturedMesh;
use crate::render_backend::render_interface::resources::{GpuBufferHandle, GpuBufferTrait};

#[cfg(target_arch = "wasm32")]
fn get_canvas_size(window: &Arc<Window>) -> (u32, u32) {
    let canvas = window.canvas().unwrap();
    // let dpr = window.device_pixel_ratio();
    let width = canvas.client_width() as u32;
    let height = canvas.client_height() as u32;
    (width, height)
}

impl WgpuRenderBackend {
    fn resize_surface_if_needed(&mut self, window: &Arc<Window>) {
        if self.size_changed {
            let mut max_x = u32::MAX;
            let mut max_y = u32::MAX;

            #[cfg(target_arch = "wasm32")]
            {
                (max_x, max_y) = get_canvas_size(window);
                self.config.width = max_x;
                self.config.height = max_y;
            }

            log::info!("canvas_size_is: {}, {}", self.config.height, self.config.width);

            self.config.width = self.size.width.min(max_x);
            self.config.height = self.size.height.min(max_y);
            
            self.surface.configure(&self.device.device, &self.config);

            for pass in self.render_passes.values_mut() {
                pass.access().depth_stencil = Texture::create_depth_texture(&self.device.as_ref().device, &self.config, "depth stencil texture")
            }
            self.size_changed = false;
        }
    }

    pub async fn new(window: &Arc<Window>) -> Self {
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
                    else { wgpu::Limits::default() }.using_resolution(adapter.limits()),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .unwrap();

        let mut size = window.inner_size();
        size.width = size.width.max(800);
        size.height = size.height.max(600);

        let mut config = surface
            .get_default_config(&adapter, size.width, size.height)
            .unwrap();
        config.format = TextureFormat::Rgba8Unorm;

        surface.configure(&device, &config);

        let egui_renderer = EguiRenderer::new(&device, config.format, None, 1, window.as_ref());

        Self {
            surface,
            _adapter: adapter,
            device: Arc::new(Device {
                device,
                queue
            }),
            config,
            size,
            size_changed: false,
            max_size: PhysicalSize {width: 800, height: 600},
            render_passes: HashMap::new(),
            egui_renderer,
        }
    }
}

impl CreateDrawletWgpu for WgpuRenderBackend
{
    fn create_render_pass(self: &mut Self) -> PassHandle {
        let id = Self::get_render_pass_id();
        let ret = rj::Own::new(WgpuRenderPass::new(
            &self.device,
            &self.config
        ));
        self.render_passes.insert(id.clone(), ret.clone());

        PassHandle { id, ptr: ret.upcast() }
    }
}

pub trait CreateDrawletWgpu
{
    fn create_render_pass(
        self: &mut Self
    ) -> PassHandle;

}

pub struct WgpuBuffer<T> {
    size: usize,
    buffer: wgpu::Buffer,
    _phantom_data: PhantomData<T>
}

impl<T> WgpuBuffer<T> {
    pub(crate) fn slice(&self) -> BufferSlice<'_> {
        self.buffer.slice(..)
    }
}

impl<T: 'static> GpuBufferTrait<T> for WgpuBuffer<T> {
    fn get_size_bytes(&self) -> usize {
        self.size * size_of::<T>()
    }
    fn get_count(&self) -> usize {
        self.size
    }
}