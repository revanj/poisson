use std::error::Error;
use winit::event_loop::{EventLoop};
use rust_renderer::{PoissonEngine};
use rust_renderer::slang;
use rust_renderer::vulkan::VulkanRenderBackend;

fn main() -> Result<(), Box<dyn Error>> {
    
    let event_loop = EventLoop::new()?;
    let _ = event_loop.run_app(PoissonEngine::<VulkanRenderBackend>::new());

    Ok(())
}
