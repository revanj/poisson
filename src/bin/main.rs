//! Simple winit window example.

use std::error::Error;

use winit::application::ApplicationHandler;
use winit::event_loop::{ActiveEventLoop, EventLoop};
#[cfg(web_platform)]
use winit::platform::web::WindowAttributesExtWeb;
use winit::raw_window_handle::HasDisplayHandle;
use winit::window::{Window, WindowAttributes};

use rust_renderer::PoissonEngine;


fn main() -> Result<(), Box<dyn Error>> {
    
    let event_loop = EventLoop::new()?;
    let _ = event_loop.run_app(PoissonEngine::new());

    Ok(())
}
