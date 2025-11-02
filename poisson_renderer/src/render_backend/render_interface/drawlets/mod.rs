pub mod colored_mesh;
pub mod textured_mesh;
mod lit_colored_mesh;

use crate::render_backend::render_interface::drawlets::colored_mesh::ColoredMesh;
use crate::render_backend::render_interface::drawlets::textured_mesh::TexturedMesh;
use crate::render_backend::render_interface::RenderObject;
use crate::render_backend::web::WgpuRenderObject;
use crate::render_backend::{DrawletID, PassID, PipelineID, RenderDrawlet};

pub(crate) trait DrawletTrait<RenObjType: RenderObject> {}



pub(crate) trait PassTrait: std::any::Any {
    fn create_textured_mesh_pipeline(&mut self, shader_path: &str, shader_text: &str) -> (PipelineID, rj::Own<dyn PipelineTrait<TexturedMesh>>);
    fn create_colored_mesh_pipeline(&mut self, shader_path: &str, shader_text: &str) -> (PipelineID, rj::Own<dyn PipelineTrait<ColoredMesh>>);
}

pub struct PassHandle {
    pub(crate) id: PassID,
    pub(crate) ptr: rj::Own<dyn PassTrait>
}

impl PassHandle {
    pub fn create_textured_mesh_pipeline(&mut self, shader_path: &str, shader_text: &str) -> PipelineHandle<TexturedMesh> {
        let (id, pipe) = self.ptr.access().create_textured_mesh_pipeline(shader_path, shader_text);
        PipelineHandle {
            id,
            ptr: pipe
        }

    }
    pub fn create_colored_mesh_pipeline(&mut self, shader_path: &str, shader_text: &str) -> PipelineHandle<ColoredMesh> {
        let (id, pipe) = self.ptr.access().create_colored_mesh_pipeline(shader_path, shader_text);
        PipelineHandle {
            id,
            ptr: pipe
        }
    }
}

pub trait PipelineTrait<RenObjType: RenderObject> {
    fn create_drawlet(&mut self, init_data: RenObjType::Data) -> rj::Own<RenObjType::DynDrawlet>;
}
pub struct PipelineHandle<RenObjType: RenderObject> {
    id: PipelineID,
    ptr: rj::Own<dyn PipelineTrait<RenObjType>>
}

impl<RenObjType: RenderObject> PipelineHandle<RenObjType> {
    pub fn create_drawlet(&mut self, init_data: RenObjType::Data) -> DrawletHandle<RenObjType> {
        let ptr_drawlet = self.ptr.access().create_drawlet(init_data);
        DrawletHandle::<RenObjType> {
            id: DrawletID(0),
            ptr: ptr_drawlet,
        }

    }
}

pub struct DrawletHandle<RenObjType: RenderObject> {
    pub(crate) id: DrawletID,
    pub(crate) ptr: rj::Own<RenObjType::DynDrawlet>
}

