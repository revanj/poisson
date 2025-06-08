//! Simple winit window example.

use std::error::Error;

use winit::application::ApplicationHandler;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::raw_window_handle::HasDisplayHandle;
use winit::window::{Window};

use rust_renderer::PoissonEngine;

use cxx;

#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("rust-renderer/src/slang/slang.h");

        type BlobstoreClient;

        fn new_blobstore_client() -> UniquePtr<BlobstoreClient>;

        fn compile() -> i32;
    }
}

fn main() -> Result<(), Box<dyn Error>> {

    let _ = ffi::compile();
    
    let event_loop = EventLoop::new()?;
    let _ = event_loop.run_app(PoissonEngine::new());

    Ok(())
}
