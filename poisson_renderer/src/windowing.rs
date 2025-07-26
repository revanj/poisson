use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{WindowAttributes, WindowId};
use crate::PoissonEngine;
use crate::render_backend::RenderBackend;

impl<Backend: RenderBackend> ApplicationHandler for PoissonEngine<Backend> {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop)
    {
        let window_attributes = WindowAttributes::default().with_resizable(true);

        self.window = match event_loop.create_window(window_attributes) {
            Ok(window) => Some(Arc::from(window)),
            Err(err) => {
                eprintln!("error creating window: {err}");
                event_loop.exit();
                return;
            },
        };

        if let Some(window_value) = self.window.clone() {
            Backend::init(self.render_backend.clone(), window_value);
        }
        
        self.init();
    }

    fn window_event(&mut self, event_loop: &dyn ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match &event {
            // those two should push event to a queue to be resolved before render loop
            WindowEvent::KeyboardInput { .. } => {
                self.render_backend.lock().as_mut().unwrap().process_event(&event);
            },
            WindowEvent::CloseRequested => {
                event_loop.exit();
            },
            WindowEvent::RedrawRequested { .. } => {
                self.window.as_ref().unwrap().pre_present_notify();
                self.update();
                self.request_redraw();
            },
            WindowEvent::SurfaceResized(PhysicalSize { width, height }) => {
                self.render_backend.lock().as_mut().unwrap().resize(*width, *height);
                self.update();
                self.request_redraw();
            },
            _ => (),
        }
        
        self.input.process_event(&event);
    }
}
