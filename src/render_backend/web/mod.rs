use std::sync::Arc;
use std::time::SystemTime;
use async_trait::async_trait;
use winit::window::Window;
use crate::render_backend::RenderBackend;
use wgpu;
use winit::dpi::PhysicalSize;

#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowExtWeb;

pub struct WgpuRenderBackend {
    surface: wgpu::Surface<'static>,
    _adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    size_changed: bool,
    max_size: winit::dpi::PhysicalSize<u32>
}


impl RenderBackend for WgpuRenderBackend {
    fn update(self: &mut Self, current_frame: usize) {
        if self.size.width == 0 || self.size.height == 0 {
            return;
        }
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
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
        }

        self.queue.submit(Some(encoder.finish()));
        output.present();
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

            // 将 canvas 添加到当前网页中
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
                // WebGL doesn't support all of wgpu's features, so if
                // we're building for the web we'll have to disable some.
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .unwrap();

        let mut size = window.surface_size();
        size.width = size.width.max(800);
        size.height = size.height.max(600);
        let config = surface
            .get_default_config(&adapter, size.width, size.height)
            .unwrap();
        surface.configure(&device, &config);

        Self {
            surface,
            _adapter: adapter,
            device,
            queue,
            config,
            size,
            size_changed: false,
            max_size: PhysicalSize {width: 800, height: 600}
        }
    }
}