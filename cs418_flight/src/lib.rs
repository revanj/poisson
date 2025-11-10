mod mesh;

use std::cell::RefCell;
use cgmath as cg;
use console_error_panic_hook;
use fs_embed::fs_embed;
use instant::Instant;
use poisson_renderer::input::{Input, KeyCode, PhysicalKey};
use poisson_renderer::math::utils::perspective;
use poisson_renderer::render_backend::web::{CreateDrawletWgpu, EguiUiShow, WgpuPipeline, WgpuRenderBackend};
use poisson_renderer::render_backend::RenderBackend;
use poisson_renderer::{init_logger, render_backend, run_game, shader, PoissonGame};
use std::error::Error;
use std::f32::consts::PI;
use std::ops::Index;
use std::rc::Rc;
use std::sync::Arc;
use std::task::Context;
use cgmath::{EuclideanSpace, InnerSpace, SquareMatrix};
use cgmath::num_traits::{Float, FloatConst};
use egui::Key::V;
use wasm_bindgen_futures::js_sys::Math::sin;
use web_sys::Document;
use poisson_renderer::render_backend::render_interface::drawlets::{DrawletHandle, PassHandle, PipelineHandle, PipelineTrait};
use poisson_renderer::render_backend::render_interface::Mesh;
use rj::Own;

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        use wasm_bindgen::prelude::*;
        use web_sys::{window, HtmlInputElement, HtmlButtonElement};
    }
}


use poisson_renderer::render_backend::render_interface::drawlets::colored_mesh::{ColoredMesh, ColoredMeshData, ColoredVertex};
use poisson_renderer::render_backend::render_interface::drawlets::lit_colored_mesh::{LitColoredMesh, LitColoredMeshData};

#[cfg_attr(target_arch="wasm32", wasm_bindgen(start))]
pub async fn run_wasm() {
    console_error_panic_hook::set_once();
    run().unwrap();
}

pub fn run() ->  Result<(), impl Error> {
    init_logger();
    run_game::<Terrain>()
}

pub struct TerrainParams {
    pub faults: usize,
    pub grid_size: usize
}

struct FlightParams {
    pos: cg::Vector3<f32>,
    yaw: f32,
    pitch: f32,
}

impl FlightParams {
    fn new() -> Self {
        Self {
            pos: cg::Vector3::new(2.5f32, 2.5f32, 2.5f32),
            yaw: 1.25f32 * f32::PI(),
            pitch: -0.25f32 * f32::PI(),
        }
    }

    pub fn turn_pitch(&mut self, speed: f32) {
        self.pitch += speed;
        if self.pitch > f32::PI() / 2.0f32 { self.pitch = f32::PI() / 2.0f32 }
        if self.pitch < -f32::PI() / 2.0f32 { self.pitch = -f32::PI() / 2.0f32 }
    }

    pub fn turn_yaw(&mut self, speed: f32) {
        self.yaw += speed;
    }

    pub fn move_front_back(&mut self, speed: f32) {
        let dir = Self::yaw_pitch_to_dir(self.yaw, self.pitch);
        self.pos += dir * speed;
    }
    pub fn move_left_right(&mut self, speed: f32) {
        let dir = Self::yaw_pitch_to_dir(self.yaw, self.pitch);
        let left = cg::Vector3::unit_y().cross(dir).normalize();
        self.pos += left * speed;
    }

    fn yaw_pitch_to_dir(yaw: f32, pitch: f32) -> cg::Vector3<f32> {
        cg::Vector3::new(
            yaw.sin(),
            pitch.sin(),
            yaw.cos()
        ).normalize()
    }

    pub fn to_view_matrix(&self) -> cg::Matrix4<f32> {
        let dir = Self::yaw_pitch_to_dir(self.yaw, self.pitch);
        cg::Matrix4::look_to_rh(
            cg::Point3::from_vec(self.pos),
            dir,
            cg::Vector3::unit_y(),
        )
    }
}

pub struct Terrain {
    //document: Option<Document>,
    terrain_mesh: Option<DrawletHandle<LitColoredMesh>>,
    scene_render_pass: Option<PassHandle>,
    lit_colored_mesh_pipeline: Option<PipelineHandle<LitColoredMesh>>,
    last_time: Instant,
    elapsed_time: f32,
    assets: fs_embed::Dir,
    egui_state: EguiState,
    terrain_params: Rc<RefCell<Option<TerrainParams>>>,
    flight_params: FlightParams,
}

impl PoissonGame for Terrain {
    type Ren = WgpuRenderBackend;

    fn new() -> Self {
        static FILES: fs_embed::Dir = fs_embed!("assets");

        Self {
            //document: None,
            scene_render_pass: None,
            lit_colored_mesh_pipeline: None,
            terrain_mesh: None,
            last_time: Instant::now(),
            elapsed_time: 0f32,
            assets: FILES.clone().auto_dynamic(),
            egui_state: EguiState {},
            terrain_params: Rc::new(RefCell::new(None)),
            flight_params: FlightParams::new(),
        }
    }

    fn pre_init(self: &mut Self, input: &mut Input) {
        input.set_mapping("move_forward", vec![PhysicalKey::Code(KeyCode::KeyW)]);
        input.set_mapping("move_back", vec![PhysicalKey::Code(KeyCode::KeyS)]);
        input.set_mapping("move_left", vec![PhysicalKey::Code(KeyCode::KeyA)]);
        input.set_mapping("move_right", vec![PhysicalKey::Code(KeyCode::KeyD)]);
        input.set_mapping("rotate_right", vec![PhysicalKey::Code(KeyCode::ArrowRight)]);
        input.set_mapping("rotate_left", vec![PhysicalKey::Code(KeyCode::ArrowLeft)]);
        input.set_mapping("rotate_up", vec![PhysicalKey::Code(KeyCode::ArrowUp)]);
        input.set_mapping("rotate_down", vec![PhysicalKey::Code(KeyCode::ArrowDown)]);
    }

    fn init(self: &mut Self, _input: &mut Input, renderer: &mut Self::Ren) {
        self.terrain_params = Rc::new(RefCell::new(Some(TerrainParams {
            faults: 50,
            grid_size: 50,
        })));

        self.last_time = Instant::now();

        let lit_colored_mesh_shader = self.assets.get_file(shader!("shaders/lit_colored_mesh")).unwrap();
        let lit_colored_mesh_shader_content = lit_colored_mesh_shader.read_str().unwrap();

        let mut r_handle = renderer.create_render_pass();

        let p_handle = r_handle.create_pipeline::<LitColoredMesh>(
            "cs418_terrain/assets/shaders/lit_colored_mesh",
            lit_colored_mesh_shader_content.as_str());

        self.scene_render_pass = Some(r_handle);
        self.lit_colored_mesh_pipeline = Some(p_handle);
    }

    fn update(self: &mut Self, input: &mut Input, renderer: &mut Self::Ren) {
        let params_submitted = self.terrain_params.borrow().is_some();
        if params_submitted {
            {
                let data = self.terrain_params.borrow();
                let data = data.as_ref().unwrap();
                let mesh_grid = mesh::mesh_grid(data.grid_size - 1, data.faults);
                let vertex_buffer = renderer.create_vertex_buffer(mesh_grid.0.as_slice());
                let index_buffer = renderer.create_index_buffer(mesh_grid.1.as_slice());
                let lit_mesh_data = LitColoredMeshData {
                    mvp_data: cg::Matrix4::identity(),
                    light_dir: cg::Vector4 {x: 1f32, y: 0f32, z: 0f32, w: 0f32},
                    view_dir: cg::Vector4 {x: 1f32, y: 0f32, z: 0f32, w: 0f32},
                    mesh: Arc::new(Mesh {
                        index: index_buffer,
                        vertex: vertex_buffer,
                    }),
                };
                if let Some(drawlet)= self.terrain_mesh.take() {
                    self.lit_colored_mesh_pipeline.as_mut().unwrap().remove_drawlet(drawlet);
                }

                self.terrain_mesh = Some(self.lit_colored_mesh_pipeline.as_mut().unwrap().create_drawlet(lit_mesh_data));
                self.terrain_mesh.as_mut().unwrap().set_light_direction(cg::Vector3::<f32>::new(2f32, 2f32, 2f32));
            }
            self.terrain_params.replace(None);
        }

        if let Some(terrain_params) = self.terrain_params.borrow().as_ref() {
            log::info!("TerrainParams: {}, {}", terrain_params.faults, terrain_params.grid_size);
        }
        let delta_time = self.last_time.elapsed().as_secs_f32();
        self.last_time = Instant::now();

        self.elapsed_time += delta_time;

        let camera_center = cgmath::Vector3::new(2f32 * self.elapsed_time.cos(), 2f32, 2f32 * self.elapsed_time.sin());
        let v = cgmath::Matrix4::look_at_rh(
            cgmath::Point3::from_vec(camera_center),
            cgmath::Point3::new(0.0, 0.0, 0.0),
            cgmath::Vector3::new(0.0, 1.0, 0.0));
        if input.is_pressed("rotate_up") { self.flight_params.turn_pitch(delta_time); }
        if input.is_pressed("rotate_down") { self.flight_params.turn_pitch(-delta_time); }
        if input.is_pressed("rotate_left") {self.flight_params.turn_yaw(delta_time); }
        if input.is_pressed("rotate_right") {self.flight_params.turn_yaw(-delta_time); }
        if input.is_pressed("move_forward") {self.flight_params.move_front_back(delta_time/2f32); }
        if input.is_pressed("move_back") {self.flight_params.move_front_back(-delta_time/2f32); }
        if input.is_pressed("move_left") { self.flight_params.move_left_right(delta_time/2f32); }
        if input.is_pressed("move_right") { self.flight_params.move_left_right(-delta_time/2f32); }

        let v = self.flight_params.to_view_matrix();
        let aspect_ratio = (renderer.get_width() as f32)/(renderer.get_height() as f32);

        let p = perspective(PI/12f32, aspect_ratio, 0.1, 100.0, Self::Ren::PERSPECTIVE_ALIGNMENT);

        if let Some(terrain_mesh) = &mut self.terrain_mesh {
            terrain_mesh.set_mvp(p * v);
            terrain_mesh.set_light_direction(cg::Vector3::<f32>::new(0.0, 1.0, -0.5));
            terrain_mesh.set_view_direction(self.flight_params.pos);
        }
    }

    fn get_egui_ui_show(self: &mut Self) -> &mut impl EguiUiShow {
        &mut self.egui_state
    }
}
struct EguiState {}
impl EguiUiShow for EguiState {
    fn show(&mut self, ctx: &egui::Context) {
        // egui::Window::new("winit + egui + wgpu says hello!")
        //     .resizable(true)
        //     .vscroll(true)
        //     .default_open(true)
        //     .show(ctx, |ui| {
        //         ui.label("Label!");
        //
        //         if ui.button("Button!").clicked() {
        //             println!("boom!")
        //         }
        //
        //         ui.separator();
        //     });
    }
}