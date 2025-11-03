#![feature(arbitrary_self_types)]

use std::any::Any;
use std::marker::PhantomData;
use std::sync::Arc;
use bytemuck::{Pod, Zeroable};
use winit::window::Window;
use parking_lot::Mutex;
use winit::event::WindowEvent;
use crate::{AsAny, PoissonGame};
use crate::egui::EguiRenderer;
use crate::render_backend::render_interface::RenderObject;
use crate::render_backend::render_interface::resources::GpuBufferHandle;
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
pub struct PassID(usize);

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct PipelineID(usize);

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct DrawletID(usize);

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct ViewID(usize);


pub trait Ren {
}
pub struct Wgpu {} impl Ren for Wgpu {}
pub struct Vk {} impl Ren for Vk {}


pub trait RenderBackend {
    const PERSPECTIVE_ALIGNMENT: [f32; 3];
    fn init(backend_clone: Arc<Mutex<Option<Self>>>, window: Arc<Window>) where Self: Sized;
    fn render(self: &mut Self, window: &Arc<Window>);
    fn process_event(self: &mut Self, window: &Window, event: &WindowEvent);
    fn resize(self: &mut Self, width: u32, height: u32);
    fn create_index_buffer(self: &Self, data: &[u32]) -> GpuBufferHandle<u32>;
    fn create_vertex_buffer<T:Sized + 'static>(self: &Self, data: &[T]) -> GpuBufferHandle<T>;

    fn get_width(self: &Self) -> u32;
    fn get_height(self: &Self) -> u32;

    fn get_egui_renderer(self: &Self) -> EguiRenderer;

    fn get_render_pass_id() -> PassID {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER:AtomicUsize = AtomicUsize::new(1);
        PassID(COUNTER.fetch_add(1, Ordering::Relaxed))
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

// #[derive(Copy, Clone)]
// pub struct PassHandle {
//     id: PassID,
// }

// #[derive(Copy, Clone)]
// pub struct PipelineHandle<D:RenderObject> {
//     id: PipelineID,
//     layer_id: PassID,
//     _pipeline_ty: PhantomData<D>
// }

// #[derive(Copy, Clone)]
// pub struct DrawletHandle<D:RenderObject> {
//     id: DrawletID,
//     pipeline_id: PipelineID,
//     layer_id: PassID,
//     _drawlet_ty: PhantomData<D>
// }

// #[derive(Copy, Clone)]
// pub struct ViewHandle {
//     id: ViewID
// }

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct MvpUniform {
    data: [[f32; 4]; 4]
}