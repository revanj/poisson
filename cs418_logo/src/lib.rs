use std::error::Error;
use std::f32::consts::PI;
use std::marker::PhantomData;
use std::sync::Arc;
use instant::Instant;
use poisson_renderer::{init_logger, run_game, shader, PoissonGame};
use console_error_panic_hook;
use poisson_renderer::input::Input;
use poisson_renderer::render_backend::{DrawletHandle, Mat4Ubo, PipelineHandle, RenderBackend, LayerHandle};
use poisson_renderer::render_backend::web::{CreateDrawletWgpu, WgpuRenderBackend};
use winit::keyboard::{KeyCode, PhysicalKey};
use cgmath as cg;
use cgmath::SquareMatrix;
use fs_embed::fs_embed;
use poisson_renderer::math::utils::{orthographic, perspective};

// #[cfg(not(target_arch = "wasm32"))]
// use poisson_renderer::render_backend::vulkan::{CreateDrawletVulkan, VulkanRenderBackend};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;
use poisson_renderer::render_backend::render_interface::{ColoredMesh, ColoredMeshData, ColoredVertex, WgpuMesh, TexturedMesh};

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
    colored_mesh_pipeline: Option<PipelineHandle<ColoredMesh>>,
    orange_mesh_inst: Option<DrawletHandle<ColoredMesh>>,
    blue_mesh_inst: Option<DrawletHandle<ColoredMesh>>,
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
            colored_mesh_pipeline: None,
            orange_mesh_inst: None,
            blue_mesh_inst: None,
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
        
        

        let index_buffer_data = vec![
            1u32, 2, 0,
            0, 2, 3,
            3, 2, 4,
            5, 4, 3,
            2, 11, 6,
            2, 6 ,5,
            6, 8, 7,
            11, 8, 6,
            10, 9, 11,
            9, 8, 11
        ];
        let mut orange_vertices = vec!{
            ColoredVertex {pos: [-3.5f32, -5.0f32, 0.0f32], color: Default::default()},
            ColoredVertex {pos: [-3.5f32, -3f32, 0.0f32], color: Default::default()},
            ColoredVertex {pos: [-1.5f32, -3f32, 0.0f32], color: Default::default()},
            ColoredVertex {pos: [3.5f32, -5f32, 0.0f32], color: Default::default()},
            ColoredVertex {pos: [3.5f32, -3f32, 0.0f32], color: Default::default()},
            ColoredVertex {pos: [1.5f32, -3f32, 0.0f32], color: Default::default()},
            ColoredVertex {pos: [1.5f32, 3f32, 0.0f32], color: Default::default()},
            ColoredVertex {pos: [3.5f32, 3f32, 0.0f32], color: Default::default()},
            ColoredVertex {pos: [3.5f32, 5f32, 0.0f32], color: Default::default()},
            ColoredVertex {pos: [-3.5f32, 5f32, 0.0f32], color: Default::default()},
            ColoredVertex {pos: [-3.5f32, 3f32, 0.0f32], color: Default::default()},
            ColoredVertex {pos: [-1.5f32, 3f32, 0.0f32], color: Default::default()},
        };

        let index_buffer = renderer.create_index_buffer(index_buffer_data.as_slice());
        let vertex_buffer = renderer.create_vertex_buffer(orange_vertices.as_slice());
        

        for vertex in &mut orange_vertices {
            vertex.color = [1f32, 0.373f32, 0.02f32];
        }

        let mut blue_vertices = orange_vertices.clone();
        
        let orange_mesh_data = ColoredMeshData {
            mvp_data: cg::Matrix4::identity(),
            mesh: Arc::new(WgpuMesh {
                index_data: index_buffer,
                vertex_data: vertex_buffer,
                num_indices: index_buffer_data.len() as u32,
                _phantom_data: PhantomData::default()
            }),
        };
     

        let triangle_shader = self.assets.get_file(shader!("shaders/colored_mesh")).unwrap();
        let triangle_shader_content = triangle_shader.read_str().unwrap();
        
        let r_handle = renderer.create_render_pass();
        let p_handle: PipelineHandle<ColoredMesh> = 
            renderer.create_pipeline(&r_handle,
                "cs418_logo/assets/shaders/colored_mesh",
                triangle_shader_content.as_str());
        
        self.orange_mesh_inst = Some(renderer.create_drawlet(&p_handle, orange_mesh_data));
        
        self.colored_mesh_pipeline = Some(p_handle);
        self.scene_render_pass = Some(r_handle);
    }

    fn update(self: &mut Self, input: &mut Input, renderer: &mut Self::Ren) {
        let delta_time = self.last_time.elapsed().as_secs_f32();
        self.last_time = Instant::now();

        //if input.is_pressed("up") {
            self.elapsed_time += delta_time;
        //}
        
        let elapsed_time = self.elapsed_time;
        
        let m_orange =
            cg::Matrix4::from_translation(cg::Vector3 {x: 400f32 + elapsed_time.cos() * 200f32, y: 300f32 + elapsed_time.sin() * 200f32,  z: 0f32})
                * cg::Matrix4::from_scale(10f32 + elapsed_time.sin() * 5f32)
                * cg::Matrix4::from_angle_z(cgmath::Deg(90.0 * elapsed_time));
        let m_blue =
            cg::Matrix4::from_translation(cg::Vector3 {x: 400f32 + elapsed_time.cos() * 200f32, y: 300f32 + elapsed_time.sin() * 200f32,  z: 0f32})
                * cg::Matrix4::from_scale(1.2f32)
                * cg::Matrix4::from_scale(10f32 + elapsed_time.sin() * 5f32)
                * cg::Matrix4::from_angle_z(cgmath::Deg(90.0 * elapsed_time));
        
        let v = cg::Matrix4::<f32>::identity();
        let p = orthographic(0f32, 800f32, 600f32, 0f32, -10f32, 10f32, Self::Ren::PERSPECTIVE_ALIGNMENT);
        let orange_ubo = Mat4Ubo { data: p * v * m_orange };
        let drawlet_orange = renderer.get_drawlet_mut(self.orange_mesh_inst.as_ref().unwrap());
        drawlet_orange.set_mvp(orange_ubo);
    }
}
