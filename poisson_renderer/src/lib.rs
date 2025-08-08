#![feature(adt_const_params)]
extern crate core;

use crate::render_backend::{CreateDrawlet, PipelineHandle, RenderPipeline, Vertex};
use env_logger;
use std::sync::Arc;
use winit::window::Window;

pub mod render_backend;
mod windowing;
pub mod input;

use console_error_panic_hook;
use parking_lot::Mutex;

use crate::render_backend::RenderBackend;

#[cfg(target_arch = "wasm32")]
use web_sys;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;
use winit::event_loop::EventLoop;
use crate::input::Input;
use crate::render_backend::web::WgpuRenderBackend;

pub trait PoissonGame {
    type Ren: RenderBackend;
    fn new() -> Self;
    fn init(self: &mut Self, input: &mut Input, renderer: &mut Self::Ren);
    fn update(self: &mut Self, input: &mut Input, renderer: &mut Self::Ren);
}



pub struct PoissonEngine<GameType>
where GameType: PoissonGame
{
    window: Option<Arc<dyn Window>>,
    input: input::Input,
    renderer: Arc<Mutex<Option<GameType::Ren>>>,
    game: GameType,
}

impl<GameType: PoissonGame> PoissonEngine<GameType>
{
    pub fn new() -> Self {
        Self {
            window: None,
            input: input::Input::new(),
            renderer: Default::default(),
            game: GameType::new(),
        }
    }
    
    fn init(self: &mut Self) {
        if let Some(backend) = self.renderer.lock().as_mut() {
            self.game.init(&mut self.input, backend);
        }
    }
    
    fn update(self: &mut Self) {
        
        if let Some(render_backend) = self.renderer.lock().as_mut() {
            self.game.update(&mut self.input, render_backend);
            render_backend.render();
        }
        
    }

    fn request_redraw(self: &mut Self) {
        self.window.as_ref()
            .expect("redraw request without a window").request_redraw();
    }

}


// #[cfg(not(target_arch = "wasm32"))]
// pub fn run_vulkan<Game: PoissonGame<RenBackend=VulkanRenderBackend>>(game: Game) -> Result<(), impl std::error::Error> {
//     let event_loop = EventLoop::new()?;
//     event_loop.run_app(PoissonEngine::<Game, _>::new())
// }

pub fn run_game<Game>() -> Result<(), impl std::error::Error>
where Game: PoissonGame
{
    let event_loop = EventLoop::new()?;
    log::info!("run app!!");
    event_loop.run_app(PoissonEngine::<Game>::new())
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
pub async fn run_wasm<Game: PoissonGame<Ren=WgpuRenderBackend>>() {
    init_logger();
    console_error_panic_hook::set_once();
    log::info!("running!!!");
    run_game::<Game>().unwrap();
}