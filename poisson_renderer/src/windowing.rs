use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{WindowAttributes, WindowId};
use crate::{PoissonEngine, PoissonGame};
use crate::render_backend::RenderBackend;

impl<GameType: PoissonGame> ApplicationHandler for PoissonEngine<GameType> where
{
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
            GameType::Ren::init(self.renderer.clone(), window_value);
        }
    }

    fn window_event(&mut self, event_loop: &dyn ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match &event {
            // those two should push event to a queue to be resolved before render_interface loop
            WindowEvent::KeyboardInput { .. } => {
                self.renderer.lock().as_mut().unwrap().process_event(&event);
            },
            WindowEvent::CloseRequested => {
                event_loop.exit();
            },
            WindowEvent::RedrawRequested { .. } => {
                self.init_or_update();
            },
            WindowEvent::SurfaceResized(PhysicalSize { width, height }) => {
                self.renderer.lock().as_mut().unwrap().resize(*width, *height);
                self.init_or_update();
            },
            _ => (),
        }
        
        self.input.process_event(&event);
    }
}

impl<GameType: PoissonGame> PoissonEngine<GameType> {
    fn init_or_update(&mut self) {
        if !self.done_init {
            self.init();
        } else {
            self.window.as_ref().unwrap().pre_present_notify();
            self.update();
        }
        self.request_redraw();
    }
}