extern crate core;

use std::any::Any;
use env_logger;
use std::sync::Arc;
use winit::window::Window;

pub mod render_backend;
mod windowing;
pub mod input;
pub mod utils;
mod game_elements;
pub mod math;

use parking_lot::Mutex;

use crate::render_backend::RenderBackend;
use winit::event_loop::EventLoop;
use crate::input::Input;

#[cfg(target_arch = "wasm32")]
use {
    console_log,
    wasm_bindgen::prelude::wasm_bindgen,
    web_sys
};

pub trait PoissonGame {
    type Ren: RenderBackend;
    fn new() -> Self;
    fn pre_init(self: &mut Self, input: &mut Input);
    fn init(self: &mut Self, input: &mut Input, renderer: &mut Self::Ren);
    fn update(self: &mut Self, input: &mut Input, renderer: &mut Self::Ren);
}

trait AsAny {
    fn as_any(self: &Self) -> &dyn Any;
    fn as_any_mut(self: &mut Self) -> &mut dyn Any;
}

pub struct PoissonEngine<GameType>
where GameType: PoissonGame
{
    window: Option<Arc<dyn Window>>,
    input: Input,
    renderer: Arc<Mutex<Option<GameType::Ren>>>,
    game: GameType,
    done_init: bool,
}

impl<GameType: PoissonGame> PoissonEngine<GameType>
{
    pub fn new() -> Self {
        let mut game = GameType::new();
        let mut input = Input::new();
        game.pre_init(&mut input);
        Self {
            window: None,
            input,
            renderer: Default::default(),
            game,
            done_init: false
        }
    }
    
    fn init(self: &mut Self) {
        if let Some(backend) = self.renderer.lock().as_mut() {
            self.game.init(&mut self.input, backend);
            self.done_init = true;
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


pub fn run_game<Game>() -> Result<(), impl std::error::Error>
where Game: PoissonGame
{
    let event_loop = EventLoop::new()?;
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