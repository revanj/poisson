use std::error::Error;
use std::f32::consts::PI;
use std::time::Instant;
use winit::keyboard::{KeyCode, PhysicalKey};
use poisson_renderer::input::Input;
use poisson_renderer::PoissonGame;
use poisson_renderer::render_backend::{CreateDrawletVulkan, CreateDrawletWgpu, DrawletHandle, PipelineHandle, RenderBackend, TexturedMesh, TexturedMeshData, Mat4Ubo, Vertex};
use poisson_renderer::render_backend::vulkan::{utils, VulkanRenderBackend};
use poisson_renderer::render_backend::web::textured_mesh::TexturedMeshDrawlet;
use poisson_renderer::render_backend::web::WgpuRenderBackend;

fn main() -> Result<(), impl Error> {
    poisson_renderer::run_game::<NothingGame>()
}

// macro_rules! ren {
//     ($x:ident) => (<<NothingGame as PoissonGame>::Ren as RenderBackend>::$x)
// }

struct NothingGame {
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
    
    fn init(self: &mut Self, input: &mut Input, renderer: &mut Self::Ren) {
        self.last_time = Instant::now();
        input.set_mapping("up", vec![PhysicalKey::Code(KeyCode::KeyW)]);
        let index_buffer_data = vec![0u32, 1, 2, 2, 3, 0];

        let vertices = vec!{
            Vertex {pos: [-0.5f32, -0.5f32, 0.0f32], tex_coord: [1.0f32, 0.0f32]},
            Vertex {pos: [0.5f32, -0.5f32, 0.0f32], tex_coord: [0.0f32, 0.0f32]},
            Vertex {pos: [0.5f32, 0.5f32, 0.0f32], tex_coord: [0.0f32, 1.0f32]},
            Vertex {pos: [-0.5f32, 0.5f32, 0.0f32], tex_coord: [1.0f32, 1.0f32]},
        };

        let diffuse_bytes = include_bytes!("../../../textures/happy-tree.png");
        let binding = image::load_from_memory(diffuse_bytes).unwrap();

        let textured_mesh_data = TexturedMeshData {
            index_data: index_buffer_data,
            vertex_data: vertices,
            texture_data: binding,
        };

        let p_handle: PipelineHandle<TexturedMesh> = renderer.create_pipeline("shaders/triangle.slang");
        self.textured_mesh_inst = Some(renderer.create_drawlet(&p_handle, textured_mesh_data));
        self.textured_mesh_pipeline = Some(p_handle);
    }

    fn update(self: &mut Self, input: &mut Input, renderer: &mut Self::Ren) {
        let delta_time = self.last_time.elapsed().as_secs_f32();
        self.last_time = Instant::now();
        
        if input.is_pressed("up") {
            self.elapsed_time += delta_time;
        }
        
        let drawlet = renderer.get_drawlet_mut(self.textured_mesh_pipeline.as_ref().unwrap(), self.textured_mesh_inst.as_ref().unwrap());

        let elapsed_time = self.elapsed_time;
        let aspect =  800f32/600f32;
        let m =  cgmath::Matrix4::from_angle_z(cgmath::Deg(90.0 * elapsed_time));
        let v = cgmath::Matrix4::look_at(
            cgmath::Point3::new(2.0, 2.0, 2.0),
            cgmath::Point3::new(0.0, 0.0, 0.0),
            cgmath::Vector3::new(0.0, 0.0, 1.0));
        let p = utils::perspective(PI/4f32, aspect, 0.1, 10.0, Self::Ren::PERSPECTIVE_ALIGNMENT);
        let new_ubo = Mat4Ubo { mvp: p * v * m };
        drawlet.set_mvp(new_ubo)
    }
}

impl NothingGame { 
    
}
