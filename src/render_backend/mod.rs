use std::sync::Arc;
use std::time::SystemTime;
use winit::window::Window;
use async_trait::async_trait;
use parking_lot::Mutex;
use crate::PoissonEngine;

pub trait RenderBackend {
    //fn init(backend_clone: Option<Arc<Mutex<Self>>>);
    fn update(self: &mut Self, current_frame: usize);
    fn resize(self: &mut Self, width: u32, height: u32);
}

#[cfg(not(target_arch = "wasm32"))]
pub mod vulkan;
pub mod web;
