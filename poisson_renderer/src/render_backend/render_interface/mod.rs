pub mod resources;
pub mod drawlets;

use std::sync::Arc;
use image::DynamicImage;
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


