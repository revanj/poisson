mod mesh;

use std::cell::RefCell;
use cgmath as cg;
use console_error_panic_hook;
use fs_embed::fs_embed;
use instant::Instant;
use poisson_renderer::input::Input;
use poisson_renderer::math::utils::perspective;
use poisson_renderer::render_backend::web::{CreateDrawletWgpu, EguiUiShow, WgpuPipeline, WgpuRenderBackend};
use poisson_renderer::render_backend::RenderBackend;
use poisson_renderer::{init_logger, render_backend, run_game, shader, PoissonGame};
use std::error::Error;
use std::f32::consts::PI;
use std::rc::Rc;
use std::sync::Arc;
use std::task::Context;
use cgmath::SquareMatrix;
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
        }
    }

    fn pre_init(self: &mut Self, input: &mut Input) {
        //input.set_mapping("up", vec![PhysicalKey::Code(KeyCode::KeyW)]);
    }

    fn init(self: &mut Self, _input: &mut Input, renderer: &mut Self::Ren) {
        cfg_if::cfg_if! {
        if #[cfg(target_arch="wasm32")] {
            let document = window().unwrap().document().unwrap();

            let grid_size: HtmlInputElement = document.get_element_by_id("gridsize").unwrap().dyn_into().unwrap();
            let faults :HtmlInputElement = document.get_element_by_id("faults").unwrap().dyn_into().unwrap();
            let button: HtmlInputElement = document.get_element_by_id("submit").unwrap().dyn_into().unwrap();

            let faults_clone = faults.clone();
            let grid_size_clone = grid_size.clone();

            let params_clone = self.terrain_params.clone();

            let closure = Closure::wrap(Box::new(move || {
                params_clone.replace(Some(TerrainParams {
                    faults: faults_clone.value().parse::<usize>().unwrap(),
                    grid_size: grid_size_clone.value().parse::<usize>().unwrap()
                }));
            }) as Box<dyn FnMut()>);

            button.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref()).unwrap();
            closure.forget();
        } else {
            self.terrain_params = Rc::new(RefCell::new(Some(TerrainParams {
                faults: 50,
                grid_size: 50,
            })))
        }}

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
                let mesh_grid = mesh::mesh_grid(data.grid_size - 1);
                let vertex_buffer = renderer.create_vertex_buffer(mesh_grid.0.as_slice());
                let index_buffer = renderer.create_index_buffer(mesh_grid.1.as_slice());
                let lit_mesh_data = LitColoredMeshData {
                    mvp_data: cg::Matrix4::identity(),
                    light_dir: cg::Vector4 {x: 1f32, y: 0f32, z: 0f32, w: 0f32},
                    mesh: Arc::new(Mesh {
                        index: index_buffer,
                        vertex: vertex_buffer,
                    }),
                };
                self.terrain_mesh = Some(self.lit_colored_mesh_pipeline.as_mut().unwrap().create_drawlet(lit_mesh_data));
            }
            self.terrain_params.replace(None);
        }

        if let Some(terrain_params) = self.terrain_params.borrow().as_ref() {
            log::info!("TerrainParams: {}, {}", terrain_params.faults, terrain_params.grid_size);
        }
        let delta_time = self.last_time.elapsed().as_secs_f32();
        self.last_time = Instant::now();

        self.elapsed_time += delta_time;

        let v = cgmath::Matrix4::look_at_rh(
            cgmath::Point3::new(2.0, 2.0, 2.0),
            cgmath::Point3::new(0.0, 0.0, 0.0),
            cgmath::Vector3::new(0.0, 1.0, 0.0));
        let aspect_ratio = (renderer.get_width() as f32)/(renderer.get_height() as f32);

        let p = perspective(PI/12f32, aspect_ratio, 0.1, 100.0, Self::Ren::PERSPECTIVE_ALIGNMENT);

        if let Some(terrain_mesh) = &mut self.terrain_mesh {
            terrain_mesh.set_mvp(p * v);
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