use std::any::Any;
use std::marker::PhantomData;
use std::sync::Arc;
use bytemuck::{Pod, Zeroable};
use image::DynamicImage;
use winit::window::Window;
use parking_lot::Mutex;
use winit::event::WindowEvent;
use crate::{AsAny, PoissonGame};
use crate::render_backend::render_interface::RenderObject;
// #[cfg(not(target_arch = "wasm32"))]
// pub mod vulkan;

pub mod web;
pub mod render_interface;


#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub struct Mat4Ubo {
    pub data: cgmath::Matrix4<f32>
}

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct LayerID(usize);

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct PipelineID(usize);

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct DrawletID(usize);

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct ViewID(usize);


pub trait RenderBackend {
    type Buffer: Buffer;
    const PERSPECTIVE_ALIGNMENT: [f32; 3];
    fn init(backend_clone: Arc<Mutex<Option<Self>>>, window: Arc<dyn Window>) where Self: Sized;
    fn render(self: &mut Self, window: &Arc<dyn Window>);
    fn process_event(self: &mut Self, event: &WindowEvent);
    fn resize(self: &mut Self, width: u32, height: u32);
    fn create_index_buffer(self: &Self, data: &[u32]) -> Self::Buffer;
    fn create_vertex_buffer<T:Sized>(self: &Self, data: &[T]) -> Self::Buffer;

    fn get_render_pass_id() -> LayerID {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER:AtomicUsize = AtomicUsize::new(1);
        LayerID(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    fn get_pipeline_id() -> PipelineID {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER:AtomicUsize = AtomicUsize::new(1);
        PipelineID(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
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

#[derive(Copy, Clone)]
pub struct LayerHandle {
    id: LayerID,
}

#[derive(Copy, Clone)]
pub struct PipelineHandle<D:RenderObject> {
    id: PipelineID,
    layer_id: LayerID,
    _pipeline_ty: PhantomData<D>
}

#[derive(Copy, Clone)]
pub struct DrawletHandle<D:RenderObject> {
    id: DrawletID,
    pipeline_id: PipelineID,
    layer_id: LayerID,
    _drawlet_ty: PhantomData<D>
}

#[derive(Copy, Clone)]
pub struct ViewHandle {
    id: ViewID
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct MvpUniform {
    data: [[f32; 4]; 4]
}

pub trait Buffer {
    fn len(&self) -> usize;
}