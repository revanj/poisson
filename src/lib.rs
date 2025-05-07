use winit::window::Window;
mod vulkan;
use vulkan::VulkanContext;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::raw_window_handle::{HasDisplayHandle, HasRawDisplayHandle, HasRawWindowHandle};
use winit::window::{WindowAttributes, WindowId};

#[path = "utils/fill.rs"]
mod fill;

pub struct PoissonEngine {
    window: Option<Box<dyn Window>>,
    vulkan_context: Option<VulkanContext>
}

impl PoissonEngine {
    pub fn new() -> Self {
        Self {
            window: None,
            vulkan_context: None
        }
    }

    pub fn init() {

    }
}

impl ApplicationHandler for PoissonEngine {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop)
    {
        let window_attributes = WindowAttributes::default();
        self.window = match event_loop.create_window(window_attributes) {
            Ok(window) => Some(window),
            Err(err) => {
                eprintln!("error creating window: {err}");
                event_loop.exit();
                return;
            },
        };

        if let Some(window_value) = &self.window {
            self.vulkan_context = Some(VulkanContext::new(
                window_value.display_handle().unwrap().as_raw()));
        }
    }

    fn window_event(&mut self, event_loop: &dyn ActiveEventLoop, _: WindowId, event: WindowEvent) {
        println!("{event:?}");
        match event {
            WindowEvent::CloseRequested => {
                println!("Close was requested; stopping");
                event_loop.exit();
            },
            WindowEvent::SurfaceResized(_) => {
                self.window.as_ref().expect("resize event without a window").request_redraw();
            },
            WindowEvent::RedrawRequested => {
                // Redraw the application.
                //
                // It's preferable for applications that do not render continuously to render in
                // this event rather than in AboutToWait, since rendering in here allows
                // the program to gracefully handle redraws requested by the OS.

                let window = self.window.as_ref()
                    .expect("redraw request without a window");
                window.pre_present_notify();
                
                fill::fill_window(window.as_ref());

                window.request_redraw();
            },
            _ => (),
        }
    }
}
