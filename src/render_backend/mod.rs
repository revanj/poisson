use std::time::SystemTime;
use winit::window::Window;

pub trait RenderBackend {
    fn new(window: &Box<dyn Window>) -> Self;
    fn update(self: &mut Self, init_time: SystemTime, current_frame: usize);
    fn resize(self: &mut Self, width: u32, height: u32);
}

pub mod vulkan;