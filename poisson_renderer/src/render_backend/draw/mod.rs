use crate::render_backend::RenderBackend;

pub mod textured_mesh;


pub struct DrawHandle {}

pub struct PipelineHandle {}

pub trait Draw<Backend: RenderBackend>
{
    fn register(self: &Self, backend: &mut Backend);
    fn draw(self: &Self, backend: &mut Backend);
}