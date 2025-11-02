use std::sync::Arc;
use image::DynamicImage;
use crate::render_backend::render_interface::drawlets::DrawletTrait;
use crate::render_backend::render_interface::{Mesh, RenderObject};

pub trait TexturedMeshDrawletTrait: DrawletTrait<TexturedMesh> {}

#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub struct UvVertex {
    pub pos: [f32; 3],
    pub tex_coord: [f32; 2]
}
pub struct TexturedMesh {}
impl RenderObject for TexturedMesh {
    type Data = TexturedMeshData;
    type DynDrawlet = dyn TexturedMeshDrawletTrait;
}
pub struct TexturedMeshData {
    pub mvp_data: cgmath::Matrix4<f32>,
    pub mesh: Arc<Mesh<UvVertex>>,
    pub texture_data: DynamicImage
}