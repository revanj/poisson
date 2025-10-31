use std::error::Error;
use std::f32::consts::PI;
use instant::Instant;
use poisson_renderer::{init_logger, run_game, shader, PoissonGame};
use console_error_panic_hook;
use poisson_renderer::input::Input;
use poisson_renderer::render_backend::{DrawletHandle, Mat4Ubo, PipelineHandle, RenderBackend, LayerHandle};
use poisson_renderer::render_backend::web::{CreateDrawletWgpu, WgpuRenderBackend};
use winit::keyboard::{KeyCode, PhysicalKey};
use cgmath;
use cgmath::{Matrix4, SquareMatrix};
use fs_embed::fs_embed;
use poisson_renderer::math::utils::perspective;

// #[cfg(not(target_arch = "wasm32"))]
// use poisson_renderer::render_backend::vulkan::{CreateDrawletVulkan, VulkanRenderBackend};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;
use poisson_renderer::render_backend::render_interface::{Mesh, TexVertex, TexturedMesh, TexturedMeshData};

#[cfg_attr(target_arch="wasm32", wasm_bindgen(start))]
pub async fn run_wasm() {
    console_error_panic_hook::set_once();
    run().unwrap();
}

pub fn run() ->  Result<(), impl Error> {
    init_logger();
    run_game::<NothingGame>()
}

pub struct NothingGame {
    scene_render_pass: Option<LayerHandle>,
    textured_mesh_pipeline: Option<PipelineHandle<TexturedMesh>>,
    textured_mesh_inst: Option<DrawletHandle<TexturedMesh>>,
    last_time: Instant,
    elapsed_time: f32,
    assets: fs_embed::Dir,
}

impl PoissonGame for NothingGame {
    type Ren = WgpuRenderBackend;

    fn new() -> Self {
        static FILES: fs_embed::Dir = fs_embed!("assets");
        Self {
            scene_render_pass: None,
            textured_mesh_pipeline: None,
            textured_mesh_inst: None,
            last_time: Instant::now(),
            elapsed_time: 0f32,
            assets: FILES.clone().auto_dynamic()
        }
    }

    fn pre_init(self: &mut Self, input: &mut Input) {
        input.set_mapping("up", vec![PhysicalKey::Code(KeyCode::KeyW)]);
    }

    fn init(self: &mut Self, _input: &mut Input, renderer: &mut Self::Ren) {
        self.last_time = Instant::now();

        let index_buffer_data = vec![0u32, 1, 2, 2, 3, 0];

        let vertices = vec!{
            TexVertex {pos: [-0.5f32, -0.5f32, 0.0f32], tex_coord: [1.0f32, 0.0f32]},
            TexVertex {pos: [0.5f32, -0.5f32, 0.0f32], tex_coord: [0.0f32, 0.0f32]},
            TexVertex {pos: [0.5f32, 0.5f32, 0.0f32], tex_coord: [0.0f32, 1.0f32]},
            TexVertex {pos: [-0.5f32, 0.5f32, 0.0f32], tex_coord: [1.0f32, 1.0f32]},
        };

        let texture_file = self.assets.get_file("textures/happy-tree.png").unwrap();
        let texture_bytes = texture_file.read_bytes().unwrap();
        let binding = image::load_from_memory(texture_bytes.as_slice()).unwrap();

        let textured_mesh_data = TexturedMeshData {
            mvp_data: Matrix4::identity(),
            mesh: Mesh {
                index: index_buffer_data,
                vertex: vertices,
            },
            texture_data: binding,
        };

        let triangle_shader = self.assets.get_file(shader!("shaders/triangle")).unwrap();
        let triangle_shader_content = triangle_shader.read_str().unwrap();
        
        let r_handle = renderer.create_render_pass();
        let p_handle: PipelineHandle<TexturedMesh> = renderer.create_pipeline(&r_handle,"nothing_game/assets/shaders/triangle", triangle_shader_content.as_str());
        self.textured_mesh_inst = Some(renderer.create_drawlet(&p_handle, textured_mesh_data));
        self.textured_mesh_pipeline = Some(p_handle);
        self.scene_render_pass = Some(r_handle);
    }

    fn update(self: &mut Self, input: &mut Input, renderer: &mut Self::Ren) {
        let delta_time = self.last_time.elapsed().as_secs_f32();
        self.last_time = Instant::now();

        if input.is_pressed("up") {
            self.elapsed_time += delta_time;
        }

        let drawlet = renderer.get_drawlet_mut(self.textured_mesh_inst.as_ref().unwrap());

        let elapsed_time = self.elapsed_time;
        let aspect =  800f32/600f32;
        let m =  cgmath::Matrix4::from_angle_z(cgmath::Deg(90.0 * elapsed_time));
        let v = cgmath::Matrix4::look_at_rh(
            cgmath::Point3::new(2.0, 2.0, 2.0),
            cgmath::Point3::new(0.0, 0.0, 0.0),
            cgmath::Vector3::new(0.0, 0.0, 1.0));
        let p = perspective(PI/4f32, aspect, 0.1, 10.0, Self::Ren::PERSPECTIVE_ALIGNMENT);
        let new_ubo = Mat4Ubo { data: p * v * m };
        drawlet.set_mvp(new_ubo)
    }
}

impl NothingGame {

}