use ash::vk;
use ash::vk::{DescriptorType, ShaderStageFlags};
use image::RgbaImage;
use crate::render_backend::draw::Draw;
use crate::render_backend::vulkan::VulkanRenderBackend;
use crate::render_backend::web::WgpuRenderBackend;



#[derive(Clone, Debug, Copy)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub color: [f32; 3],
    pub tex_coord: [f32; 2]
}

#[derive(Clone, Debug, Copy)]
pub struct UniformBufferObject {
    pub model: cgmath::Matrix4<f32>,
    pub view: cgmath::Matrix4<f32>,
    pub proj: cgmath::Matrix4<f32>,
}

pub struct TexturedMesh {
    shader_bytecode: Vec<u32>
}

impl TexturedMesh {

}
