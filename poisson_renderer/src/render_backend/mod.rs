use std::any::Any;
use std::marker::PhantomData;
use std::sync::{Arc, Weak};
use bytemuck::{Pod, Zeroable};
use image::DynamicImage;
use winit::window::Window;
use parking_lot::Mutex;
use winit::event::WindowEvent;
use crate::{AsAny, PoissonEngine, PoissonGame};
use crate::input::Input;

#[cfg(not(target_arch = "wasm32"))]
pub mod vulkan;

pub mod web;
pub mod math;

#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub tex_coord: [f32; 2]
}

#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub struct Mat4Ubo {
    pub mvp: cgmath::Matrix4<f32>
}

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct PipelineID(usize);

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct DrawletID(usize);


pub trait RenderBackend {
    const PERSPECTIVE_ALIGNMENT: [f32; 3];
    fn init(backend_clone: Arc<Mutex<Option<Self>>>, window: Arc<dyn Window>) where Self: Sized;
    fn render(self: &mut Self);
    fn process_event(self: &mut Self, event: &WindowEvent);
    fn resize(self: &mut Self, width: u32, height: u32);
}

pub trait RenderPipeline<RenObj: RenderObject> {
    fn get_drawlet_id() -> DrawletID {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER:AtomicUsize = AtomicUsize::new(1);
        DrawletID(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

pub trait RenderDrawlet: Sized {
    type Data;
}

pub struct PipelineHandle<D:RenderObject> {
    id: PipelineID,
    _pipeline_ty: PhantomData<D>
}

pub struct DrawletHandle<D:RenderObject> {
    id: DrawletID,
    _drawlet_ty: PhantomData<D>
}


pub struct TexturedMeshData {
    pub index_data: Vec<u32>,
    pub vertex_data: Vec<Vertex>,
    pub texture_data: DynamicImage
}

pub struct TexturedMesh {}

pub trait RenderObject {}

impl RenderObject for TexturedMesh {}


