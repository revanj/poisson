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
    vulkan_context: Option<VulkanContext>,
}

impl PoissonEngine {
    pub fn new() -> Self {
        Self {
            window: None,
            vulkan_context: None
        }
    }

    fn init(self: &mut Self) {
        if let Some(window_value) = &self.window {
            unsafe {
                self.vulkan_context = Some(VulkanContext::new(window_value));
            }
        }
    }

    fn update(self: &mut Self) {

    }

    fn pre_present_notify(self: &mut Self) {
        self.window.as_ref()
            .expect("redraw request without a window").pre_present_notify();
    }

    fn request_redraw(self: &mut Self) {
        self.window.as_ref()
            .expect("redraw request without a window").request_redraw();
    }

    fn render_loop(self: &mut Self) {
        let window = self.window.as_ref()
            .expect("redraw request without a window").as_ref();
    }

    fn present(self: &mut Self) {

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

        self.init();
    }

    fn window_event(&mut self, event_loop: &dyn ActiveEventLoop, _: WindowId, event: WindowEvent) {
        println!("{event:?}");
        match event {
            // those two should push event to a queue to be resolved before render loop
            WindowEvent::KeyboardInput { .. } => {},
            WindowEvent::PointerButton { .. } => {},

            WindowEvent::CloseRequested => {
                println!("Close was requested; stopping");
                event_loop.exit();
            },
            WindowEvent::SurfaceResized(_) => {
                self.vulkan_context.as_mut().unwrap().notify_window_resized();
                self.window.as_ref().expect("resize event without a window").request_redraw();
            },
            WindowEvent::RedrawRequested => {
                self.update();
                self.render_loop();
                self.pre_present_notify();
                self.present();
                self.request_redraw()
            },
            _ => (),
        }
    }
}
