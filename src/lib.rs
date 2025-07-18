use core::time;
use std::time::{SystemTime, UNIX_EPOCH};
use ash::vk;
use winit::window::Window;
pub mod vulkan;
pub mod slang;

use vulkan::VulkanContext;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::raw_window_handle::{HasDisplayHandle, HasRawDisplayHandle, HasRawWindowHandle};
use winit::window::{WindowAttributes, WindowId};
use winit::dpi::PhysicalSize;
use winit::event_loop::ControlFlow::Poll;
use crate::vulkan::render_object::UniformBufferObject;

#[allow(clippy::too_many_arguments)]
pub fn record_submit_commandbuffer<F: FnOnce(&ash::Device, vk::CommandBuffer)>(
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    in_flight_fence: &vk::Fence,
    submit_queue: vk::Queue,
    wait_mask: &[vk::PipelineStageFlags],
    wait_semaphores: &[vk::Semaphore],
    signal_semaphores: &[vk::Semaphore],
    f: F,
) {
    unsafe {
        device
            .reset_command_buffer(
                command_buffer,
                vk::CommandBufferResetFlags::RELEASE_RESOURCES,
            )
            .expect("Reset command buffer failed.");

        let command_buffer_begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        device
            .begin_command_buffer(command_buffer, &command_buffer_begin_info)
            .expect("Begin commandbuffer");
        f(device, command_buffer);
        device
            .end_command_buffer(command_buffer)
            .expect("End commandbuffer");

        let command_buffers = vec![command_buffer];

        let submit_info = vk::SubmitInfo::default()
            .wait_semaphores(wait_semaphores)
            .wait_dst_stage_mask(wait_mask)
            .command_buffers(&command_buffers)
            .signal_semaphores(signal_semaphores);

        device
            .queue_submit(submit_queue, &[submit_info], *in_flight_fence)
            .expect("queue submit failed.");
    }
}

pub trait RenderBackend {
    fn new(window: &Box<dyn Window>) -> Self;
    fn update(self: &mut Self, init_time: SystemTime, current_frame: usize);
    fn resize(self: &mut Self, width: u32, height: u32);
}

pub struct VulkanBackend {
    pub vulkan_context: VulkanContext
}
impl VulkanBackend {
    fn update_uniform_buffer(vulkan_context: &mut VulkanContext, current_frame: usize, elapsed_time: f32) {
        let res = vulkan_context.physical_surface.surface_resolution;
        let aspect = res.width as f32 / res.height as f32;
        let new_ubo = [UniformBufferObject {
            model: cgmath::Matrix4::from_angle_z(cgmath::Deg(90.0 * elapsed_time)),
            view: cgmath::Matrix4::look_at(
                cgmath::Point3::new(2.0, 2.0, 2.0),
                cgmath::Point3::new(0.0, 0.0, 0.0),
                cgmath::Vector3::new(0.0, 0.0, 1.0),
            ),
            proj: vulkan::utils::perspective(cgmath::Deg(45.0), aspect, 0.1, 10.0),
        }];
        vulkan_context.uniform_buffers[current_frame].write(&new_ubo);
    }
}
impl RenderBackend for VulkanBackend {
    
    fn new(window: &Box<dyn Window>) -> Self{
        Self {
            vulkan_context: unsafe { VulkanContext::new(window) }
        }
    }
    
    fn update(self: &mut Self, init_time: SystemTime, current_frame: usize) {
        let vulkan = &mut self.vulkan_context;
        unsafe {
            vulkan.device.device.wait_for_fences(
                &[vulkan.frames_in_flight_fences[current_frame]],
                true, u64::MAX).unwrap();
        }

        if let Some(extent) = vulkan.new_swapchain_size {
            if extent.width <= 0 || extent.height <= 0 {
                return;
            }
            unsafe { vulkan.recreate_swapchain(extent)};
            vulkan.new_swapchain_size = None;
        }

        let viewports = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: vulkan.physical_surface.resolution().width as f32,
            height: vulkan.physical_surface.resolution().height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];
        let scissors = [vulkan.physical_surface.resolution().into()];

        unsafe {vulkan.device.device.reset_fences(&[vulkan.frames_in_flight_fences[current_frame]]).unwrap()};

        let acquire_result = unsafe {vulkan
            .swapchain.swapchain_loader
            .acquire_next_image(
                vulkan.swapchain.swapchain,
                u64::MAX,
                vulkan.image_available_semaphores[current_frame],
                vk::Fence::null())};

        let present_index = match acquire_result {
            Ok((present_index, _)) => present_index,
            _ => panic!("Failed to acquire swapchain."),
        };


        let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 0.0],
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            },
        ];

        let render_pass_begin_info = vk::RenderPassBeginInfo::default()
            .render_pass(vulkan.render_pass.render_pass)
            .framebuffer(vulkan.framebuffers[present_index as usize].framebuffer)
            .render_area(vulkan.physical_surface.resolution().into())
            .clear_values(&clear_values);

        let elapsed_time = SystemTime::now().duration_since(init_time).unwrap().as_secs_f32();

        println!("elapsed time is {}", elapsed_time);

        Self::update_uniform_buffer(vulkan, current_frame, elapsed_time);

        record_submit_commandbuffer(
            &vulkan.device.device,
            vulkan.draw_command_buffers.command_buffers[current_frame],
            &vulkan.frames_in_flight_fences[current_frame],
            vulkan.device.present_queue,
            &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT],
            &[vulkan.image_available_semaphores[current_frame]],
            &[vulkan.rendering_complete_semaphores[present_index as usize]],
            |device, draw_command_buffer| {
                unsafe { device.cmd_begin_render_pass(
                    draw_command_buffer,
                    &render_pass_begin_info,
                    vk::SubpassContents::INLINE,
                );
                    device.cmd_bind_pipeline(
                        draw_command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        vulkan.graphics_pipeline,
                    );
                    device.cmd_set_viewport(draw_command_buffer, 0, &viewports);
                    device.cmd_set_scissor(draw_command_buffer, 0, &scissors);
                    device.cmd_bind_vertex_buffers(
                        draw_command_buffer,
                        0,
                        &[vulkan.vertex_buffer.buffer],
                        &[0],
                    );
                    device.cmd_bind_index_buffer(
                        draw_command_buffer,
                        vulkan.index_buffer.buffer,
                        0,
                        vk::IndexType::UINT32,
                    );
                    device.cmd_bind_descriptor_sets(draw_command_buffer, vk::PipelineBindPoint::GRAPHICS, vulkan.pipeline_layout, 0, vulkan.descriptor_sets[current_frame..current_frame+1].as_ref(), &[]);
                    device.cmd_draw_indexed(
                        draw_command_buffer,
                        3, // index_buffer_data.len() as u32, #TODO: change this to a variable
                        1,
                        0,
                        0,
                        1,
                    );
                    // Or draw without the index buffer
                    // device.cmd_draw(draw_command_buffer, 3, 1, 0, 0);
                    device.cmd_end_render_pass(draw_command_buffer);}
            },
        );
        let signal_semaphores = [vulkan.rendering_complete_semaphores[present_index as usize]];
        let swapchains = [vulkan.swapchain.swapchain];
        let image_indices = [present_index];
        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&signal_semaphores) // &base.rendering_complete_semaphore)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        unsafe {
            vulkan.swapchain.swapchain_loader
                .queue_present(vulkan.device.present_queue, &present_info)
                .unwrap()};
    }

    fn resize(self: &mut Self, width: u32, height: u32) {
        self.vulkan_context.new_swapchain_size = Some(vk::Extent2D {width, height });
    }
}


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
