use std::sync::Arc;
use image::Rgba;
use parking_lot::Mutex;
use crate::render_backend::render_interface::{ColoredMesh, ColoredMeshData, RenderObject, TexturedMesh};
use crate::render_backend::{DrawletHandle, DrawletID, Mat4Ubo, PassID, PipelineID, RenderDrawlet, Wgpu};
use crate::render_backend::web::colored_mesh::ColoredMeshDrawlet;
use crate::render_backend::web::WgpuRenderObject;

pub trait DrawletTrait<RenObjType: RenderObject> {}
pub trait TexturedMeshDrawletTrait: DrawletTrait<TexturedMesh> {}
pub trait ColoredMeshDrawletTrait: DrawletTrait<ColoredMesh> {
    fn set_mvp(self: &mut Self, mvp: cgmath::Matrix4<f32>);
}

pub trait PassTrait: std::any::Any {
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

impl PipelineHandle<ColoredMesh> {
    pub fn create_drawlet(&mut self, init_data: ColoredMeshData) -> ColoredMeshDrawletHandle {
        let ptr_drawlet = self.ptr.access().create_drawlet(init_data);
        ColoredMeshDrawletHandle {
            id: DrawletID(0),
            ptr: ptr_drawlet,
        }

    }
}

pub struct ColoredMeshDrawletHandle {
    pub(crate) id: DrawletID,
    pub(crate) ptr: rj::Own<dyn ColoredMeshDrawletTrait>
}

impl ColoredMeshDrawletHandle {
    pub fn set_mvp(self: &mut Self, mvp: cgmath::Matrix4<f32>) {
        self.ptr.access().set_mvp(mvp);
    }
}