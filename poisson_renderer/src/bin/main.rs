use std::error::Error;
use winit::event_loop::{EventLoop};
use rust_renderer::{PoissonEngine};
use rust_renderer::slang;

use rust_renderer::render_backend::vulkan::VulkanRenderBackend;

fn main() -> Result<(), impl Error> {
    rust_renderer::run_wgpu()
}
