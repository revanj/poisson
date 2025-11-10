mod mesh;

use std::cell::{Ref, RefCell};
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
use std::num::FpCategory::Normal;
use std::ops::Index;
use std::rc::Rc;
use std::sync::Arc;
use std::task::Context;
use cgmath::{EuclideanSpace, SquareMatrix};
use cgmath::num_traits::ToPrimitive;
use egui::TextBuffer;
use image::{DynamicImage, RgbaImage};
use regex::Regex;

#[cfg(target_arch = "wasm32")]
use web_sys::{CanvasRenderingContext2d, Document, Event, HtmlCanvasElement, HtmlImageElement};

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
use poisson_renderer::render_backend::render_interface::drawlets::lit_colored_mesh::{LitColoredMesh, LitColoredMeshData, NormalColoredVertex};
use poisson_renderer::render_backend::render_interface::drawlets::textured_mesh::{TexturedMesh, TexturedMeshData, UvVertex};
use poisson_renderer::render_backend::web::textured_mesh::TexturedMeshPipeline;
use crate::TextureColor::{Color, Texture};

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

pub enum TextureColor {
    Texture(RgbaImage),
    Color((f32, f32, f32, f32)),
}

#[cfg(target_arch = "wasm32")]
pub fn html_image_to_rgba(img: &HtmlImageElement) -> RgbaImage {
    let document = web_sys::window().unwrap().document().unwrap();

    // Create an offscreen canvas
    let canvas: HtmlCanvasElement = document
        .create_element("canvas").unwrap()
        .dyn_into().unwrap();

    canvas.set_width(img.width());
    canvas.set_height(img.height());

    let ctx: CanvasRenderingContext2d = canvas
        .get_context("2d").unwrap()
        .unwrap()
        .dyn_into().unwrap();

    ctx.draw_image_with_html_image_element(img, 0.0, 0.0).expect("failed to draw");

    let image_data = ctx.get_image_data(0.0, 0.0, img.width() as f64, img.height() as f64).unwrap();
    let data = image_data.data();

    let mut pixels = vec![0u8; data.len()];
    pixels.copy_from_slice(data.as_slice());

    let rgba_image = RgbaImage::from_raw(img.width(), img.height(), pixels).expect("Failed to create image");

    rgba_image
}

enum ColoredOrTexturedMesh {
    ColoredMesh(DrawletHandle<LitColoredMesh>),
    TexturedMesh(DrawletHandle<TexturedMesh>),
}

pub struct Terrain {
    //document: Option<Document>,
    terrain_mesh: Option<ColoredOrTexturedMesh>,
    scene_render_pass: Option<PassHandle>,
    lit_colored_mesh_pipeline: Option<PipelineHandle<LitColoredMesh>>,
    textured_mesh_pipeline: Option<PipelineHandle<TexturedMesh>>,
    last_time: Instant,
    elapsed_time: f32,
    assets: fs_embed::Dir,
    egui_state: EguiState,
    terrain_params: Rc<RefCell<Option<TerrainParams>>>,
    texture_color: Rc<RefCell<TextureColor>>,
    texture_color_updated: Rc<RefCell<bool>>,
    texture_vertex_list: Vec<UvVertex>,
    color_vertex_list: Vec<NormalColoredVertex>,
    index_list: Vec<u32>,
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
            texture_color: Rc::new(RefCell::new(Color((1f32, 1f32, 1f32, 0.3f32)))),
            texture_color_updated: Rc::new(RefCell::new(false)),
            texture_vertex_list: Vec::new(),
            color_vertex_list: Vec::new(),
            textured_mesh_pipeline: None,
            index_list: Vec::new(),
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
                let faults: HtmlInputElement = document.get_element_by_id("faults").unwrap().dyn_into().unwrap();
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

                // let url = "assets/textures/happy-tree.png";
                let img = HtmlImageElement::new().unwrap();
                img.set_cross_origin(Some("anonymous"));
                // img.set_src(url);

                let texture_color_clone = Rc::clone(&self.texture_color);
                let texture_color_updated_clone = Rc::clone(&self.texture_color_updated);
                let on_error = Closure::wrap(Box::new(move |_event: Event| {
                    let original_val = texture_color_clone.replace(TextureColor::Color((1.0f32, 0f32, 1f32, 0f32)));
                    if let Color((1.0f32, 0f32, 1f32, 0f32)) = original_val {
                    } else {
                       texture_color_updated_clone.replace(true);
                    }
                }) as Box<dyn FnMut(_)>);
                let texture_text_input: HtmlInputElement = document.get_element_by_id("texture").unwrap().dyn_into().unwrap();
                let texture_text_input_clone = texture_text_input.clone();
                let texture_color_updated_clone = Rc::clone(&self.texture_color_updated);
                let img_clone = img.clone();
                let texture_color_clone = Rc::clone(&self.texture_color);
                let on_load = Closure::wrap(Box::new(move |_event: Event| {
                    let rgba_image = html_image_to_rgba(&img_clone);
                    texture_color_clone.replace(TextureColor::Texture(rgba_image));
                    texture_color_updated_clone.replace(true);
                }) as Box<dyn FnMut(_)>);
                img.add_event_listener_with_callback("load", on_load.as_ref().unchecked_ref()).unwrap();
                on_load.forget();

                img.add_event_listener_with_callback("error", on_error.as_ref().unchecked_ref()).unwrap();
                on_error.forget();

                //let self_texture_text_clone = Rc::clone(&self.texture_text);
                let img_clone = img.clone();
                let texture_color_updated_clone = self.texture_color_updated.clone();
                let texture_color_clone = Rc::clone(&self.texture_color);
                let texture_text_closure = Closure::wrap(Box::new(move || {
                    let input_str = texture_text_input_clone.value();
                    if input_str == "" {
                        let original_value = texture_color_clone.replace(Color((1f32, 1f32, 1f32, 0.3f32)));
                        if let Color((1f32, 1f32, 1f32, 0.3f32)) = original_value {
                        } else {
                            log::info!("got default color, setting replace true");
                            texture_color_updated_clone.replace(true);
                        }
                    }
                    else if Regex::new(r"^#[0-9a-f]{8}$").unwrap().is_match(input_str.as_str()) {
                        let r = i32::from_str_radix(&input_str[1..3], 16).unwrap().to_f32().unwrap() / 255f32;
                        let g = i32::from_str_radix(&input_str[3..5], 16).unwrap().to_f32().unwrap() / 255f32;
                        let b = i32::from_str_radix(&input_str[5..7], 16).unwrap().to_f32().unwrap() / 255f32;
                        let a = i32::from_str_radix(&input_str[7..9], 16).unwrap().to_f32().unwrap() / 255f32;
                        log::info!("regex matched color {}, {}, {}, {}", r, g, b, a);
                        texture_color_clone.replace(Color((r, g, b, a)));
                        texture_color_updated_clone.replace(true);
                    } // else matched image
                    else if Regex::new(r"[.](jpg|png)$").unwrap().is_match(input_str.as_str())  {
                        img_clone.set_src(input_str.as_str());
                        texture_color_updated_clone.replace(true);
                    }
                }) as Box<dyn FnMut()>);

                texture_text_input.add_event_listener_with_callback("input",texture_text_closure.as_ref().unchecked_ref()).unwrap();
                texture_text_closure.forget();
            } else {
                self.terrain_params = Rc::new(RefCell::new(Some(TerrainParams {
                    faults: 50,
                    grid_size: 50,
                })))
            }
        }

        self.last_time = Instant::now();

        let lit_colored_mesh_shader = self.assets.get_file(shader!("shaders/lit_colored_mesh")).unwrap();
        let lit_colored_mesh_shader_content = lit_colored_mesh_shader.read_str().unwrap();

        let mut r_handle = renderer.create_render_pass();

        let p_handle = r_handle.create_pipeline::<LitColoredMesh>(
            "cs418_texture/assets/shaders/lit_colored_mesh",
            lit_colored_mesh_shader_content.as_str());

        let textured_mesh_shader = self.assets.get_file(shader!("shaders/textured_mesh")).unwrap();
        let textured_mesh_shader_content = textured_mesh_shader.read_str().unwrap();
        let textured_mesh_pipeline = r_handle.create_pipeline::<TexturedMesh>(
            "cs418_texture/assets/shaders/textured_mesh",
                textured_mesh_shader_content.as_str());

        self.scene_render_pass = Some(r_handle);
        self.lit_colored_mesh_pipeline = Some(p_handle);
        self.textured_mesh_pipeline = Some(textured_mesh_pipeline);
    }

    fn update(self: &mut Self, input: &mut Input, renderer: &mut Self::Ren) {
        let params_submitted = self.terrain_params.borrow().is_some();
        if params_submitted {
            {
                let data = self.terrain_params.borrow();
                let data = data.as_ref().unwrap();
                let mesh_grid = mesh::mesh_grid(data.grid_size - 1, data.faults, false);
                self.texture_vertex_list = Vec::new();
                for vertex in &mesh_grid.0 {
                    self.texture_vertex_list.push(
                        UvVertex {
                            pos: vertex.pos,
                            tex_coord: vertex.uv,
                        }
                    )
                }
                self.index_list = mesh_grid.1;

                self.color_vertex_list = Vec::new();
                for vertex in &mesh_grid.0 {
                    self.color_vertex_list.push(
                        NormalColoredVertex {
                            pos: vertex.pos,
                            color: [1.0; 4],
                            normal: vertex.normal,
                        }
                    )
                }

                match &(*self.texture_color.borrow()) {
                    Texture(tex) => {
                        let vertex_buffer = renderer.create_vertex_buffer(self.texture_vertex_list.as_slice());
                        let index_buffer = renderer.create_index_buffer(self.index_list.as_slice());
                        let lit_mesh_data = TexturedMeshData {
                            mvp_data: cg::Matrix4::identity(),
                            // light_dir: cg::Vector4 {x: 1f32, y: 0f32, z: 0f32, w: 0f32},
                            // view_dir: cg::Vector4 {x: 1f32, y: 0f32, z: 0f32, w: 0f32},
                            mesh: Arc::new(Mesh {
                                index: index_buffer,
                                vertex: vertex_buffer,
                            }),
                            texture_data: DynamicImage::ImageRgba8(tex.clone()),
                        };

                        if let Some(drawlet)= self.terrain_mesh.take() {
                            match drawlet {
                                ColoredOrTexturedMesh::ColoredMesh(drawlet) => { self.lit_colored_mesh_pipeline.as_mut().unwrap().remove_drawlet(drawlet); }
                                ColoredOrTexturedMesh::TexturedMesh(drawlet) => { self.textured_mesh_pipeline.as_mut().unwrap().remove_drawlet(drawlet); }
                            }
                        }
                        self.terrain_mesh = Some(ColoredOrTexturedMesh::TexturedMesh(
                            self.textured_mesh_pipeline.as_mut().unwrap().create_drawlet(lit_mesh_data)
                        ));
                    }
                    Color((r, g, b, a)) => {
                        log::info!("setting vertex color, {}, {}, {}, {}", r, g, b, a);
                        for vertex in &mut self.color_vertex_list {
                            vertex.color = [*r, *g, *b, *a];
                        }

                        let vertex_buffer = renderer.create_vertex_buffer(self.color_vertex_list.as_slice());
                        let index_buffer = renderer.create_index_buffer(self.index_list.as_slice());
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
                            match drawlet {
                                ColoredOrTexturedMesh::ColoredMesh(drawlet) => { self.lit_colored_mesh_pipeline.as_mut().unwrap().remove_drawlet(drawlet); }
                                ColoredOrTexturedMesh::TexturedMesh(drawlet) => { self.textured_mesh_pipeline.as_mut().unwrap().remove_drawlet(drawlet); }
                            }
                        }
                        self.terrain_mesh = Some(ColoredOrTexturedMesh::ColoredMesh(
                            self.lit_colored_mesh_pipeline.as_mut().unwrap().create_drawlet(lit_mesh_data)
                        ));
                    }
                }
            }
            self.terrain_params.replace(None);
        }

        if *self.texture_color_updated.borrow() && self.index_list.len() > 0 {
            match &(*self.texture_color.borrow()) {
                Texture(tex) => {
                    log::info!("found image of size {}, {}", tex.width(), tex.height());

                    let vertex_buffer = renderer.create_vertex_buffer(self.texture_vertex_list.as_slice());
                    let index_buffer = renderer.create_index_buffer(self.index_list.as_slice());
                    let lit_mesh_data = TexturedMeshData {
                        mvp_data: cg::Matrix4::identity(),
                        mesh: Arc::new(Mesh {
                            index: index_buffer,
                            vertex: vertex_buffer,
                        }),
                        texture_data: DynamicImage::ImageRgba8(tex.clone()),
                    };

                    if let Some(drawlet)= self.terrain_mesh.take() {
                        match drawlet {
                            ColoredOrTexturedMesh::ColoredMesh(drawlet) => { self.lit_colored_mesh_pipeline.as_mut().unwrap().remove_drawlet(drawlet); }
                            ColoredOrTexturedMesh::TexturedMesh(drawlet) => { self.textured_mesh_pipeline.as_mut().unwrap().remove_drawlet(drawlet); }
                        }
                    }
                    self.terrain_mesh = Some(ColoredOrTexturedMesh::TexturedMesh(
                        self.textured_mesh_pipeline.as_mut().unwrap().create_drawlet(lit_mesh_data)
                    ));
                }
                Color((r, g, b, a)) => {
                    log::info!("loaded color {}, {}, {}, {}", r, g, b, a);
                    for vertex in &mut self.color_vertex_list {
                        vertex.color = [*r, *g, *b, *a];
                    }
                    let vertex_buffer = renderer.create_vertex_buffer(self.color_vertex_list.as_slice());
                    let index_buffer = renderer.create_index_buffer(self.index_list.as_slice());
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
                        match drawlet {
                            ColoredOrTexturedMesh::ColoredMesh(drawlet) => { self.lit_colored_mesh_pipeline.as_mut().unwrap().remove_drawlet(drawlet); }
                            ColoredOrTexturedMesh::TexturedMesh(drawlet) => { self.textured_mesh_pipeline.as_mut().unwrap().remove_drawlet(drawlet); }
                        }
                    }
                    self.terrain_mesh = Some(ColoredOrTexturedMesh::ColoredMesh(
                        self.lit_colored_mesh_pipeline.as_mut().unwrap().create_drawlet(lit_mesh_data)
                    ));
                }
                _ => {}
            }
        }
        self.texture_color_updated.replace(false);

        let delta_time = self.last_time.elapsed().as_secs_f32();
        self.last_time = Instant::now();

        self.elapsed_time += delta_time;

        let camera_center = cgmath::Vector3::new(3f32 * self.elapsed_time.cos(), 3f32, 3f32 * self.elapsed_time.sin());

        let v = cgmath::Matrix4::look_at_rh(
            cgmath::Point3::from_vec(camera_center),
            cgmath::Point3::new(0.0, -1.0, 0.0),
            cgmath::Vector3::new(0.0, 1.0, 0.0));
        let aspect_ratio = (renderer.get_width() as f32)/(renderer.get_height() as f32);

        let p = perspective(PI/12f32, aspect_ratio, 0.1, 100.0, Self::Ren::PERSPECTIVE_ALIGNMENT);

        if let Some(terrain_mesh) = &mut self.terrain_mesh {
            match terrain_mesh {
                ColoredOrTexturedMesh::ColoredMesh(terrain_mesh) => {
                    terrain_mesh.set_mvp(p * v);
                    terrain_mesh.set_light_direction(cg::Vector3::<f32>::new(0.0, 1.0, -0.5));
                    terrain_mesh.set_view_direction(camera_center);
                }
                ColoredOrTexturedMesh::TexturedMesh(terrain_mesh) => {
                    terrain_mesh.set_mvp(p * v);
                }
            }
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