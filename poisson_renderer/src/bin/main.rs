use std::error::Error;
use winit::keyboard::{KeyCode, PhysicalKey};
use poisson_renderer::input::Input;
use poisson_renderer::PoissonGame;
use poisson_renderer::render_backend::{CreatePipeline, DrawletHandle, PipelineHandle, Vertex};
use poisson_renderer::render_backend::vulkan::render_object::{TexturedMesh, TexturedMeshDrawletData, TexturedMeshPipeline};
use poisson_renderer::render_backend::vulkan::VulkanRenderBackend;

fn main() -> Result<(), impl Error> {
    poisson_renderer::run_vulkan::<NothingGame>(NothingGame::new())
}

struct NothingGame {
    textured_mesh_pipeline: Option<PipelineHandle<TexturedMeshPipeline>>,
    textured_mesh_inst: Option<DrawletHandle<TexturedMesh>>
}

impl PoissonGame<VulkanRenderBackend> for NothingGame {
    fn new() -> Self {
        Self {
            textured_mesh_pipeline: None,
            textured_mesh_inst: None
        }
    }
    
    fn init(self: &mut Self, input: &mut Input, renderer: &mut VulkanRenderBackend) {
        input.set_mapping("up", vec![PhysicalKey::Code(KeyCode::KeyW)]);
        let index_buffer_data = vec![0u32, 1, 2, 2, 3, 0];

        let vertices = vec!{
            Vertex {pos: [-0.5f32, -0.5f32, 0.0f32],  color: [1.0f32, 0.0f32, 0.0f32], tex_coord: [1.0f32, 0.0f32]},
            Vertex {pos: [0.5f32, -0.5f32, 0.0f32],  color: [0.0f32, 1.0f32, 0.0f32], tex_coord: [0.0f32, 0.0f32]},
            Vertex {pos: [0.5f32, 0.5f32, 0.0f32],  color: [0.0f32, 0.0f32, 1.0f32], tex_coord: [0.0f32, 1.0f32]},
            Vertex {pos: [-0.5f32, 0.5f32, 0.0f32],  color: [1.0f32, 1.0f32, 1.0f32], tex_coord: [1.0f32, 1.0f32]},
        };

        let diffuse_bytes = include_bytes!("../../../textures/happy-tree.png");
        let binding = image::load_from_memory(diffuse_bytes).unwrap();

        let textured_mesh_data = TexturedMeshDrawletData {
            index_data: index_buffer_data,
            vertex_data: vertices,
            texture_data: binding,
        };

        let p_handle: PipelineHandle<TexturedMeshPipeline> = renderer.create_pipeline("shaders/triangle.slang");
        self.textured_mesh_inst = Some(renderer.create_drawlet(&p_handle, textured_mesh_data));
        self.textured_mesh_pipeline = Some(p_handle);
    }

    fn update(self: &mut Self, input: &mut Input, renderer: &mut VulkanRenderBackend) {
        if input.is_pressed("up") {
            println!("pressing up!");
        }
    }
}

impl NothingGame {
    
}
