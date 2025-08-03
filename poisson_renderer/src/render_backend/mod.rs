use std::marker::ConstParamTy;
use std::sync::{Arc, Weak};
use std::time::SystemTime;
use winit::window::Window;
use async_trait::async_trait;
use parking_lot::Mutex;
use winit::event::WindowEvent;
use crate::PoissonEngine;
use crate::render_backend::vulkan::render_object::{Bind, Draw, Inst};

pub trait DrawletHandle {
    type DrawletType: Draw;
}

pub trait PipelineHandle {
    type PipelineType: Inst;
    type DrawletType: Draw;
    
    fn get_id(self: &Self) -> PipelineID;
}



#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct PipelineID(usize);

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct DrawletID(usize);

pub trait RenderBackend {
    fn init(backend_clone: Arc<Mutex<Option<Self>>>, window: Arc<dyn Window>) where Self: Sized;
    fn render(self: &mut Self);
    fn process_event(self: &mut Self, event: &WindowEvent);
    fn resize(self: &mut Self, width: u32, height: u32);
    fn create_pipeline<PipelineType: Inst + Bind + 'static>(self: &mut Self) -> impl PipelineHandle;
    fn create_drawlet<InstType: Inst + 'static>(self: &mut Self, pipeline_id: PipelineID, init_data: &InstType::DrawletDataType) -> InstType::DrawletHandleType;
}

#[cfg(not(target_arch = "wasm32"))]
pub mod vulkan;
pub mod web;
pub(crate) mod draw;
