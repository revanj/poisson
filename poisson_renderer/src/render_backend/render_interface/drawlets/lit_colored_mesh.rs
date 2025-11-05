use std::sync::Arc;
use crate::render_backend::render_interface::drawlets::{DrawletHandle, DrawletTrait};
use crate::render_backend::render_interface::{Mesh, RenderObject};

#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub struct NormalColoredVertex {
    pub pos: [f32; 3],
    pub color: [f32; 3],
    pub normal: [f32; 3],
}

pub trait LitColoredMeshDrawletTrait: DrawletTrait<LitColoredMesh> {
    fn set_mvp(self: &mut Self, mvp: cgmath::Matrix4<f32>);
    fn set_light_dir(self: &mut Self, light_dir: cgmath::Vector3<f32>);
}

impl DrawletHandle<LitColoredMesh> {
    pub fn set_mvp(self: &mut Self, mvp: cgmath::Matrix4<f32>) {
        self.ptr.access().set_mvp(mvp);
    }
    pub fn set_light_direction(self: &mut Self, light_dir: cgmath::Vector3<f32>) {
        self.ptr.access().set_light_dir(light_dir);
    }
}

pub struct LitColoredMesh {}
impl RenderObject for LitColoredMesh {
    type Data = LitColoredMeshData;
    type DynDrawlet = dyn LitColoredMeshDrawletTrait;
}
pub struct LitColoredMeshData {
    pub mvp_data: cgmath::Matrix4<f32>,
    pub light_dir: cgmath::Vector4<f32>,
    pub mesh: Arc<Mesh<NormalColoredVertex>>
}