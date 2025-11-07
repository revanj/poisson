pub mod colored_mesh;
pub mod textured_mesh;
pub mod lit_colored_mesh;

use crate::render_backend::render_interface::drawlets::colored_mesh::ColoredMesh;
use crate::render_backend::render_interface::drawlets::textured_mesh::TexturedMesh;
use crate::render_backend::render_interface::RenderObject;
use crate::render_backend::{DrawletID, PassID, PipelineID};
use crate::render_backend::render_interface::drawlets::lit_colored_mesh::LitColoredMesh;

pub trait DrawletTrait<RenObjType: RenderObject> {}


pub trait CreatePipeline<T>  where T: RenderObject {
    fn create_pipeline(&mut self, shader_path: &str, shader_text: &str)
        -> (PipelineID, rj::Own<(dyn PipelineTrait<T> + 'static)>);
}

pub trait PassTrait: std::any::Any +
    CreatePipeline<TexturedMesh> +
    CreatePipeline<ColoredMesh> +
    CreatePipeline<LitColoredMesh>
{}

pub struct PassHandle {
    pub(crate) id: PassID,
    pub(crate) ptr: rj::Own<dyn PassTrait>
}

impl PassHandle {
    pub fn create_pipeline<T: RenderObject>(&mut self, shader_path: &str, shader_text: &str)
        -> PipelineHandle<T>
        where (dyn PassTrait + 'static): CreatePipeline<T>
    {
        let (id, pipe) = self.ptr.access().create_pipeline(shader_path, shader_text);
        PipelineHandle {
            id,
            ptr: pipe
        }
    }
}

pub trait PipelineTrait<RenObjType: RenderObject> {
    fn create_drawlet(&mut self, init_data: RenObjType::Data) -> (DrawletID, rj::Own<RenObjType::DynDrawlet>);
    fn remove_drawlet(&mut self, drawlet: DrawletHandle<RenObjType>);
}
pub struct PipelineHandle<RenObjType: RenderObject> {
    id: PipelineID,
    ptr: rj::Own<dyn PipelineTrait<RenObjType>>
}

impl<RenObjType: RenderObject> PipelineHandle<RenObjType> {
    pub fn create_drawlet(&mut self, init_data: RenObjType::Data) -> DrawletHandle<RenObjType> {
        let (id, ptr_drawlet) = self.ptr.access().create_drawlet(init_data);
        DrawletHandle::<RenObjType> {
            id,
            ptr: ptr_drawlet,
        }
    }

    pub fn remove_drawlet(&mut self, drawlet: DrawletHandle<RenObjType>) {
        self.ptr.access().remove_drawlet(drawlet);
    }
}

pub struct DrawletHandle<RenObjType: RenderObject> {
    pub(crate) id: DrawletID,
    pub(crate) ptr: rj::Own<RenObjType::DynDrawlet>
}

