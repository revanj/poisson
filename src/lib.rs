use core::time;
use std::time::{SystemTime, UNIX_EPOCH};
use ash::vk;
use winit::window::Window;
pub mod vulkan;
pub mod slang;

use vulkan::VulkanRenderBackend;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::raw_window_handle::{HasDisplayHandle, HasRawDisplayHandle, HasRawWindowHandle};
use winit::window::{WindowAttributes, WindowId};
use winit::dpi::PhysicalSize;
use winit::event_loop::ControlFlow::Poll;
use crate::vulkan::render_object::UniformBufferObject;



pub trait RenderBackend {
    fn new(window: &Box<dyn Window>) -> Self;
    fn update(self: &mut Self, init_time: SystemTime, current_frame: usize);
    fn resize(self: &mut Self, width: u32, height: u32);
}

// pub struct VulkanBackend {
//     pub vulkan_context: VulkanRenderBackend
// }
// 
// impl RenderBackend for VulkanBackend {
//     
//     fn new(window: &Box<dyn Window>) -> Self{
//         Self {
//             vulkan_context: VulkanRenderBackend::new(window)
//         }
//     }
//     
//     fn update(self: &mut Self, init_time: SystemTime, current_frame: usize) {
//         self.vulkan_context.update(init_time, current_frame);
//     }
// 
//     fn resize(self: &mut Self, width: u32, height: u32) {
//         self.vulkan_context.resize(width, height);
//     }
// }


pub struct PoissonEngine<Backend: RenderBackend> {
    window: Option<Box<dyn Window>>,
    vulkan_backend: Option<Backend>,
    current_frame: usize,
    init_time: std::time::SystemTime,
}

impl<Backend: RenderBackend> PoissonEngine<Backend> {
    pub fn new() -> Self {
        Self {
            window: None,
            vulkan_backend: None,
            current_frame: 0,
            init_time: SystemTime::now(),
        }
    }
    

    fn init(self: &mut Self) {
        self.init_time = SystemTime::now();
        if let Some(window_value) = &self.window {
            unsafe {
                self.vulkan_backend = Some(Backend::new(window_value));
            }
        }
    }

    fn update(self: &mut Self) {
        self.vulkan_backend.as_mut().unwrap().update(self.init_time, self.current_frame);
        self.current_frame += 1;
        self.current_frame = self.current_frame % 3;
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
        // let window = self.window.as_ref()
        //     .expect("redraw request without a window").as_ref();
    }

    fn present(self: &mut Self) {

    }
}

impl<Backend: RenderBackend> ApplicationHandler for PoissonEngine<Backend> {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop)
    {
        event_loop.set_control_flow(Poll);
        let window_attributes = WindowAttributes::default().with_resizable(true);

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
        match event {
            // those two should push event to a queue to be resolved before render loop
            WindowEvent::KeyboardInput { .. } => {},
            WindowEvent::CloseRequested => {
                event_loop.exit();
            },
            WindowEvent::RedrawRequested { .. } => {
                #[cfg(target_os = "windows")]
                {
                    self.update();
                    self.request_redraw();
                }
            },
            WindowEvent::SurfaceResized(PhysicalSize { width, height }) => {
                self.vulkan_backend.as_mut().unwrap().resize(width, height);
                self.update();
                self.request_redraw();
            },
            _ => (),
        }
    }

    // in linux the frame is driven from about_to_wait
    #[cfg(target_os = "linux")]
    fn about_to_wait(&mut self, event_loop: &dyn ActiveEventLoop) {
        println!("about to wait");
        self.update();
    }
}
