use std::sync::Arc;
use crate::render_backend::render_interface::drawlets::{ColoredMeshDrawletTrait, DrawletHandle};
use crate::render_backend::render_interface::{Mesh, RenderObject};

impl DrawletHandle<ColoredMesh> {
    pub fn set_mvp(self: &mut Self, mvp: cgmath::Matrix4<f32>) {
        self.ptr.access().set_mvp(mvp);
    }
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


