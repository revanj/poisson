use core::time;
use std::ops::Deref;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use winit::window::Window;
use env_logger;

use wasm_bindgen::prelude::wasm_bindgen;


pub mod render_backend;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::raw_window_handle::{HasDisplayHandle, HasRawDisplayHandle, HasRawWindowHandle};
use winit::window::{WindowAttributes, WindowId};
use winit::dpi::PhysicalSize;
use winit::event_loop::ControlFlow::Poll;

use parking_lot::Mutex;
use fern;
use console_log;
use console_error_panic_hook;
use log::logger;

use wgpu::Face::Back;
use crate::render_backend::RenderBackend;

#[cfg(not(target_arch = "wasm32"))]
use crate::render_backend::vulkan::VulkanRenderBackend;


#[cfg(target_arch = "wasm32")]
use web_sys;

use crate::render_backend::web::WgpuRenderBackend;

pub struct PoissonEngine<Backend: RenderBackend> {
    window: Option<Arc<dyn Window>>,
    render_backend: Arc<Mutex<Option<Backend>>>,
    #[allow(dead_code)]
    missed_resize: Arc<Mutex<Option<PhysicalSize<u32>>>>,
    current_frame: usize,
}


impl<Backend: RenderBackend> PoissonEngine<Backend> {
    pub fn new() -> Self {
        Self {
            window: None,
            render_backend: Default::default(),
            current_frame: 0,
            missed_resize: Default::default(),
        }
    }

    

    fn update(self: &mut Self) {
        if let Some(render_backend) = self.render_backend.lock().as_mut() {
            render_backend.update(self.current_frame);
            self.current_frame += 1;
            self.current_frame = self.current_frame % 3;
        }
    }

    fn request_redraw(self: &mut Self) {
        self.window.as_ref()
            .expect("redraw request without a window").request_redraw();
    }

}

impl ApplicationHandler for PoissonEngine<WgpuRenderBackend> {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        log::info!("can_create_surfaces!!");
        event_loop.set_control_flow(Poll);
        let window_attributes = WindowAttributes::default().with_resizable(true);
        self.window = match event_loop.create_window(window_attributes) {
            Ok(window) => Some(Arc::from(window)),
            Err(err) => {
                eprintln!("error creating window: {err}");
                event_loop.exit();
                return;
            },
        };
        
        log::info!("after creating window!");
        if let Some(window_value) = &self.window {
            cfg_if::cfg_if! {
                if #[cfg(target_arch = "wasm32")] {
                    let backend = self.render_backend.clone();
                    let missed_resize = self.missed_resize.clone();
                    let window_cloned = window_value.clone();
                    wasm_bindgen_futures::spawn_local(async move {
                        let new_backend = WgpuRenderBackend::new(&window_cloned).await;
                        let mut locked_backend = backend.lock();
                        *locked_backend = Some(new_backend);

                        if let Some(PhysicalSize{width, height}) = *missed_resize.lock() {
                            locked_backend.as_mut().unwrap().resize(width, height);
                            window_cloned.request_redraw();
                        }
                    });
                } else {
                    let render_backend = pollster::block_on(WgpuRenderBackend::new(window_value));
                    self.render_backend.lock().replace(render_backend);
                }
            }
        }
    }

    // fn resumed(&mut self, event_loop: &dyn ActiveEventLoop) {
    //     log::info!("resumed!!");
    //     if self.render_backend.as_ref().lock().is_some() {
    //         return;
    //     }
    //
    //     // event_loop.set_control_flow(Poll);
    //     let window_attributes = WindowAttributes::default().with_resizable(true);
    //     self.window = match event_loop.create_window(window_attributes) {
    //         Ok(window) => Some(Arc::from(window)),
    //         Err(err) => {
    //             eprintln!("error creating window: {err}");
    //             event_loop.exit();
    //             return;
    //         },
    //     };
    //
    //     self.init_time = SystemTime::now();
    //     if let Some(window_value) = &self.window {
    //         cfg_if::cfg_if! {
    //             if #[cfg(target_arch = "wasm32")] {
    //                 let backend = self.render_backend.clone();
    //                 let missed_resize = self.missed_resize.clone();
    //                 let window_cloned = window_value.clone();
    //                 wasm_bindgen_futures::spawn_local(async move {
    //                     let new_backend = WgpuRenderBackend::new(&window_cloned).await;
    //                     let mut locked_backend = backend.lock();
    //                     *locked_backend = Some(new_backend);
    //
    //                     if let Some(PhysicalSize{width, height}) = *missed_resize.lock() {
    //                         locked_backend.as_mut().unwrap().resize(width, height);
    //                         window_cloned.request_redraw();
    //                     }
    //                 });
    //             } else {
    //                 let render_backend = pollster::block_on(WgpuRenderBackend::new(window_value));
    //                 self.render_backend.lock().replace(render_backend);
    //             }
    //         }
    //     }
    // }

    fn window_event(&mut self, event_loop: &dyn ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            // those two should push event to a queue to be resolved before render loop
            WindowEvent::KeyboardInput { .. } => {},
            WindowEvent::CloseRequested => {
                event_loop.exit();
            },
            WindowEvent::RedrawRequested { .. } => {
                #[cfg(any(target_arch = "wasm32", target_os = "windows"))]
                {
                    self.window.as_ref().unwrap().pre_present_notify();
                    self.update();
                    self.request_redraw();
                }
            },
            WindowEvent::SurfaceResized(PhysicalSize { width, height }) => {
                self.render_backend.lock().as_mut().unwrap().resize(width, height);
                self.update();
                self.request_redraw();
            },
            _ => (),
        }
    }

    // in linux the frame is driven from about_to_wait
    // #[cfg(target_os = "linux")]
    fn about_to_wait(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.window.as_ref().unwrap().pre_present_notify();
        self.update();
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl ApplicationHandler for PoissonEngine<VulkanRenderBackend> {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop)
    {
        event_loop.set_control_flow(Poll);
        let window_attributes = WindowAttributes::default().with_resizable(true);

        self.window = match event_loop.create_window(window_attributes) {
            Ok(window) => Some(Arc::from(window)),
            Err(err) => {
                eprintln!("error creating window: {err}");
                event_loop.exit();
                return;
            },
        };
        
        if let Some(window_value) = &self.window {

            let render_backend = VulkanRenderBackend::new(window_value);
            self.render_backend.lock().replace(render_backend);

        }
    }

    fn window_event(&mut self, event_loop: &dyn ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            // those two should push event to a queue to be resolved before render loop
            WindowEvent::KeyboardInput { .. } => {},
            WindowEvent::CloseRequested => {
                event_loop.exit();
            },
            WindowEvent::RedrawRequested { .. } => {
                #[cfg(any(target_arch = "wasm32", target_os = "windows"))]
                {
                    self.window.as_ref().unwrap().pre_present_notify();
                    self.update();
                    self.request_redraw();
                }
            },
            WindowEvent::SurfaceResized(PhysicalSize { width, height }) => {
                self.render_backend.lock().as_mut().unwrap().resize(width, height);
                self.update();
                self.request_redraw();
            },
            _ => (),
        }
    }

    // in linux the frame is driven from about_to_wait
    #[cfg(target_os = "linux")]
    fn about_to_wait(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.window.as_ref().unwrap().pre_present_notify();
        self.update();
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run_vulkan() -> Result<(), impl std::error::Error> {
    let event_loop = EventLoop::new()?;
    event_loop.run_app(PoissonEngine::<VulkanRenderBackend>::new())
}

pub fn run_wgpu() -> Result<(), impl std::error::Error> {
    let event_loop = EventLoop::new()?;
    log::info!("run app!!");
    event_loop.run_app(PoissonEngine::<WgpuRenderBackend>::new())
}

pub fn init_logger() {
    cfg_if::cfg_if! {
        if #[cfg(any(target_arch = "wasm32"))] {
            let query_string = web_sys::window().unwrap().location().search().unwrap();
            let query_level: Option<log::LevelFilter> = parse_url_query_string(&query_string, "RUST_LOG")
                .and_then(|x| x.parse().ok());
            
            let base_level = query_level.unwrap_or(log::LevelFilter::Info);
            let wgpu_level = query_level.unwrap_or(log::LevelFilter::Error);
            
            fern::Dispatch::new()
                .level(base_level)
                .level_for("wgpu_core", wgpu_level)
                .level_for("wgpu_hal", wgpu_level)
                .level_for("naga", wgpu_level)
                .chain(fern::Output::call(console_log::log))
                .apply()
                .unwrap();
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        } else if #[cfg(target_os = "android")] {
            android_logger::init_once(
                android_logger::Config::default()
                    .with_max_level(log::LevelFilter::Info)
            );
            log_panics::init();
        } else {
            env_logger::builder()
                .filter_level(log::LevelFilter::Info)
                .filter_module("wgpu_core", log::LevelFilter::Info)
                .filter_module("wgpu_hal", log::LevelFilter::Error)
                .filter_module("naga", log::LevelFilter::Error)
                .parse_default_env()
                .init();
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn parse_url_query_string<'a>(query: &'a str, search_key: &str) -> Option<&'a str> {
    let query_string = query.strip_prefix('?')?;

    for pair in query_string.split('&') {
        let mut pair = pair.split('=');
        let key = pair.next()?;
        let value = pair.next()?;

        if key == search_key {
            return Some(value);
        }
    }
    None
}

#[cfg_attr(target_arch="wasm32", wasm_bindgen(start))]
pub async fn run() {
    init_logger();
    console_error_panic_hook::set_once();
    log::info!("running!!!");
    run_wgpu().unwrap();
}