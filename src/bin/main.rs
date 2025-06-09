use std::error::Error;
use winit::event_loop::{EventLoop};
use rust_renderer::PoissonEngine;
use rust_renderer::slang;


fn main() -> Result<(), Box<dyn Error>> {

    let compiler = slang::SlangCompiler::new();
    compiler.load_module("shaders/hello-world.slang");
    
    let event_loop = EventLoop::new()?;
    let _ = event_loop.run_app(PoissonEngine::new());

    Ok(())
}
