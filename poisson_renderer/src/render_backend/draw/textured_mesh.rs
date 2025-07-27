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

impl Draw<VulkanRenderBackend> for TexturedMesh {
    fn register(self: &Self, backend: &mut VulkanRenderBackend) {
        let ubo_layout_binding = vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(ShaderStageFlags::VERTEX);
        let sampler_layout_binding = vk::DescriptorSetLayoutBinding::default()
            .binding(1)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT);

        let bindings = [
            ubo_layout_binding, sampler_layout_binding];

        let descriptor_set_layout_create_info =
            vk::DescriptorSetLayoutCreateInfo::default()
                .bindings(&bindings);

        let descriptor_set_layout = unsafe {
            backend.device.device.create_descriptor_set_layout(&descriptor_set_layout_create_info, None).unwrap()
        };
        
    }

    fn draw(self: &Self, backend: &mut VulkanRenderBackend) {
        todo!()
    }
}

impl Draw<WgpuRenderBackend> for TexturedMesh {
    fn register(self: &Self, backend: &mut WgpuRenderBackend) {
        todo!()
    }

    fn draw(self: &Self, backend: &mut WgpuRenderBackend) {
        todo!()
    }
}