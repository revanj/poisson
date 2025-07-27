use std::sync::Arc;
use std::time::SystemTime;
use winit::window::Window;
use async_trait::async_trait;
use parking_lot::Mutex;
use winit::event::WindowEvent;
use crate::PoissonEngine;

pub trait RenderBackend{
    fn init(backend_clone: Arc<Mutex<Option<Self>>>, window: Arc<dyn Window>) where Self: Sized;
    fn render(self: &mut Self);
    fn process_event(self: &mut Self, event: &WindowEvent);
    fn resize(self: &mut Self, width: u32, height: u32);
}

#[cfg(not(target_arch = "wasm32"))]
pub mod vulkan;
pub mod web;
mod draw;
