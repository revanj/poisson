#![feature(adt_const_params)]
#![feature(unboxed_closures)]
extern crate core;

use std::any::Any;
use std::f32::consts::PI;
use crate::render_backend::{DrawletHandle, Mat4Ubo, PipelineHandle, RenderPipeline, TexturedMesh, TexturedMeshData, Vertex};
use env_logger;
use std::sync::Arc;
use instant::Instant;
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
use winit::keyboard::{KeyCode, PhysicalKey};
use crate::input::Input;
use crate::render_backend::math::utils::perspective;
use crate::render_backend::web::{CreateDrawletWgpu, WgpuRenderBackend};

#[cfg(not(target_arch="wasm32"))]
macro_rules! include_shader {
    ($x:expr) => {
        include_str!(concat!($x, ".slang"))
    }     
}

#[cfg(target_arch="wasm32")]
macro_rules! include_shader {
    ($x:expr) => {
        include_str!(concat!($x, ".wgsl"))
    }     
}



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
    input: input::Input,
    renderer: Arc<Mutex<Option<GameType::Ren>>>,
    game: GameType,
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
        }
    }
    
    fn init(self: &mut Self) {
        log::info!("running init function");
        if let Some(backend) = self.renderer.lock().as_mut() {
            log::info!("some backend");
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
pub async fn run_wasm() {
    init_logger();
    console_error_panic_hook::set_once();
    log::info!("running!!!");
    run_game::<NothingGame>().unwrap();
}


pub struct NothingGame {
    textured_mesh_pipeline: Option<PipelineHandle<TexturedMesh>>,
    textured_mesh_inst: Option<DrawletHandle<TexturedMesh>>,
    last_time: Instant,
    elapsed_time: f32,
}



impl PoissonGame for NothingGame {

    type Ren = WgpuRenderBackend;

    fn new() -> Self {
        Self {
            textured_mesh_pipeline: None,
            textured_mesh_inst: None,
            last_time: Instant::now(),
            elapsed_time: 0f32,
        }
    }

    fn pre_init(self: &mut Self, input: &mut Input) {
        input.set_mapping("up", vec![PhysicalKey::Code(KeyCode::KeyW)]);
    }

    fn init(self: &mut Self, input: &mut Input, renderer: &mut Self::Ren) {
        
        log::info!("attempting to print shader");
        
        self.last_time = Instant::now();

        let index_buffer_data = vec![0u32, 1, 2, 2, 3, 0];

        let vertices = vec!{
            Vertex {pos: [-0.5f32, -0.5f32, 0.0f32], tex_coord: [1.0f32, 0.0f32]},
            Vertex {pos: [0.5f32, -0.5f32, 0.0f32], tex_coord: [0.0f32, 0.0f32]},
            Vertex {pos: [0.5f32, 0.5f32, 0.0f32], tex_coord: [0.0f32, 1.0f32]},
            Vertex {pos: [-0.5f32, 0.5f32, 0.0f32], tex_coord: [1.0f32, 1.0f32]},
        };

        let diffuse_bytes = include_bytes!("../../textures/happy-tree.png");
        let binding = image::load_from_memory(diffuse_bytes).unwrap();

        let textured_mesh_data = TexturedMeshData {
            index_data: index_buffer_data,
            vertex_data: vertices,
            texture_data: binding,
        };

        let triangle_shader = include_shader!("../../shaders/triangle");

        
        log::info!("{}", triangle_shader);

        // let p_handle: PipelineHandle<TexturedMesh> = renderer.create_pipeline("shaders/triangle", triangle_shader);
        // self.textured_mesh_inst = Some(renderer.create_drawlet(&p_handle, textured_mesh_data));
        // self.textured_mesh_pipeline = Some(p_handle);
    }

    fn update(self: &mut Self, input: &mut Input, renderer: &mut Self::Ren) {
        // let delta_time = self.last_time.elapsed().as_secs_f32();
        // self.last_time = Instant::now();
        //
        // if input.is_pressed("up") {
        //     self.elapsed_time += delta_time;
        // }
        //
        // let drawlet = renderer.get_drawlet_mut(self.textured_mesh_pipeline.as_ref().unwrap(), self.textured_mesh_inst.as_ref().unwrap());
        //
        // let elapsed_time = self.elapsed_time;
        // let aspect =  800f32/600f32;
        // let m =  cgmath::Matrix4::from_angle_z(cgmath::Deg(90.0 * elapsed_time));
        // let v = cgmath::Matrix4::look_at(
        //     cgmath::Point3::new(2.0, 2.0, 2.0),
        //     cgmath::Point3::new(0.0, 0.0, 0.0),
        //     cgmath::Vector3::new(0.0, 0.0, 1.0));
        // let p = perspective(PI/4f32, aspect, 0.1, 10.0, Self::Ren::PERSPECTIVE_ALIGNMENT);
        // let new_ubo = Mat4Ubo { mvp: p * v * m };
        // drawlet.set_mvp(new_ubo)
    }
}

impl NothingGame {

}