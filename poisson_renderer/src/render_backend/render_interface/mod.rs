pub mod resources;
pub mod drawlets;

use std::sync::Arc;
use image::DynamicImage;
use crate::render_backend::DrawletHandle;
use crate::render_backend::render_interface::drawlets::{ColoredMeshDrawletTrait, DrawletTrait, TexturedMeshDrawletTrait};
use crate::render_backend::render_interface::resources::GpuBufferHandle;
use crate::render_backend::web::WgpuBuffer;

pub trait RenderObject {
    type Data;
    type DynDrawlet: ?Sized;
}

pub struct Mesh<T> {
    pub index: GpuBufferHandle<u32>,
    pub vertex: GpuBufferHandle<T>,
}

#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub struct TexVertex {
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
    pub mesh: Arc<Mesh<TexVertex>>,
    pub texture_data: DynamicImage
}

#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub struct ColoredVertex {
    pub pos: [f32; 3],
    pub color: [f32; 3]
}
pub struct ColoredMesh {}
impl RenderObject for ColoredMesh {
    type Data = ColoredMeshData;
    type DynDrawlet = dyn ColoredMeshDrawletTrait;
}
pub struct ColoredMeshData {
    pub mvp_data: cgmath::Matrix4<f32>,
    pub mesh: Arc<Mesh<ColoredVertex>>
}



