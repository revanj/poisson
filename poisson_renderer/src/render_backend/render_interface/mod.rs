pub mod resources;
pub mod drawlets;

use crate::render_backend::render_interface::resources::GpuBufferHandle;

pub trait RenderObject {
    type Data;
    type DynDrawlet: ?Sized;
}

pub struct Mesh<T> {
    pub index: GpuBufferHandle<u32>,
    pub vertex: GpuBufferHandle<T>,
}


