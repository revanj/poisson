use core::time;
use std::ops::Deref;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use winit::window::Window;
use env_logger;

use wasm_bindgen::prelude::wasm_bindgen;


pub mod render_backend;
mod windowing;
mod input;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::raw_window_handle::{HasDisplayHandle, HasRawDisplayHandle, HasRawWindowHandle};
use winit::window::{WindowAttributes, WindowId};
use winit::dpi::PhysicalSize;

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
use winit::keyboard::{KeyCode, PhysicalKey};
use crate::render_backend::web::WgpuRenderBackend;

pub struct PoissonEngine<Backend: RenderBackend> {
    window: Option<Arc<dyn Window>>,
    input: input::Input,
    render_backend: Arc<Mutex<Option<Backend>>>,
}


impl<Backend: RenderBackend> PoissonEngine<Backend> {
    pub fn new() -> Self {
        Self {
            window: None,
            input: input::Input::new(),
            render_backend: Default::default(),
        }
    }
    
    fn init(self: &mut Self) {
        self.input.set_mapping("up", vec![PhysicalKey::Code(KeyCode::KeyW)]);
    }
    
    fn update(self: &mut Self) {
        
        if let Some(render_backend) = self.render_backend.lock().as_mut() {
            render_backend.render();
        }
        
        if self.input.is_pressed("up") {
            println!("pressing up!");
        }
    }

    fn request_redraw(self: &mut Self) {
        self.window.as_ref()
            .expect("redraw request without a window").request_redraw();
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