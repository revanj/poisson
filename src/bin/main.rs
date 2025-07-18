use std::error::Error;
use winit::event_loop::{EventLoop};
use rust_renderer::{PoissonEngine, VulkanBackend};
use rust_renderer::slang;


fn main() -> Result<(), Box<dyn Error>> {
    
    let event_loop = EventLoop::new()?;
    let _ = event_loop.run_app(PoissonEngine::<VulkanBackend>::new());

    Ok(())
}
