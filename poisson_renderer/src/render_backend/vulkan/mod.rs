mod img;
mod instance;
mod physical_surface;
pub(crate) mod device;
mod swapchain;
mod command_buffer;
mod framebuffer;
pub(crate) mod render_pass;
pub mod render_object;
mod buffer;
mod physical_device;
mod texture;
mod pipeline;


use std::collections::HashMap;
use std::marker::PhantomData;
pub use instance::*;
use std::ops::Drop;
use ash::vk;

use std::mem::ManuallyDrop;

use std::sync::Arc;
use ash::vk::CommandBuffer;
use cgmath::{Matrix4, Vector4};
use parking_lot::Mutex;
use winit::event::WindowEvent;
use winit::window::Window;

use slang_refl;

use crate::{render_backend, AsAny, PoissonGame};
use render_backend::RenderBackend;
use render_backend::vulkan::command_buffer::{CommandBuffers};
use render_backend::vulkan::device::Device;
use render_backend::vulkan::framebuffer::Framebuffer;
use render_backend::vulkan::physical_surface::PhysicalSurface;
use render_backend::vulkan::swapchain::Swapchain;
use render_backend::vulkan::render_pass::RenderPass;

use vk::PipelineStageFlags;
use crate::input::Input;
use crate::render_backend::{PipelineID, DrawletHandle, PipelineHandle, RenderPipeline, RenderDrawlet, RenderObject};
use crate::render_backend::vulkan::render_object::TexturedMeshDrawlet;

pub fn find_memorytype_index(
    memory_req: &vk::MemoryRequirements,
    memory_prop: &vk::PhysicalDeviceMemoryProperties,
    flags: vk::MemoryPropertyFlags,
) -> Option<u32> {
    memory_prop.memory_types[..memory_prop.memory_type_count as _]
        .iter()
        .enumerate()
        .find(|(index, memory_type)| {
            (1u32 << index) & memory_req.memory_type_bits != 0
                && memory_type.property_flags & flags == flags
        })
        .map(|(index, _memory_type)| index as _)
}

pub trait VulkanRenderObject: RenderObject + Sized {
    type Drawlet: VulkanDrawlet;
    type Pipeline: VulkanPipeline<Self> + VulkanPipelineDyn + 'static;
    type Data;
}

pub trait VulkanPipelineDyn: AsAny {
    fn get_pipeline(self: &Self) -> vk::Pipeline;
    fn get_instances(self: &Self) -> Box<dyn Iterator<Item=&dyn VulkanDrawletDyn> + '_>;
    fn get_instances_mut(self: &mut Self) -> Box<dyn Iterator<Item=&mut dyn VulkanDrawletDyn> + '_>;
}

pub trait VulkanPipeline<RenObjType: VulkanRenderObject>: RenderPipeline<RenObjType> + VulkanPipelineDyn {
    fn instantiate_drawlet(
        self: &mut Self,
        init_data: <<RenObjType as VulkanRenderObject>::Drawlet as RenderDrawlet>::Data
    ) -> DrawletHandle<RenObjType>;

    fn get_drawlet_mut(self: &mut Self, drawlet_handle: &DrawletHandle<RenObjType>) -> &'_ mut RenObjType::Drawlet ;

    fn new(device: &Arc<Device>,
           render_pass: &RenderPass,
           shader_bytecode: &[u32],
           resolution: vk::Extent2D,
           n_framebuffers: usize,
    ) -> Self where Self: Sized;
}

pub trait VulkanDrawlet: RenderDrawlet {
    fn draw(self: &Self, command_buffer: CommandBuffer);
}
pub trait VulkanDrawletDyn {
    fn draw(self: &Self, command_buffer: CommandBuffer);
}

impl<T> VulkanDrawletDyn for T where T: VulkanDrawlet {
    fn draw(self: &Self, command_buffer: CommandBuffer) {
        self.draw(command_buffer);
    }
}

/// Vulkan Context which contains physical device, logical device, and surface, etc.
pub struct VulkanRenderBackend {
    pub instance: ManuallyDrop<Instance>,
    pub physical_surface: ManuallyDrop<PhysicalSurface>,
    pub device : ManuallyDrop<Arc<Device>>,
    pub swapchain: ManuallyDrop<Swapchain>,
    pub new_swapchain_size: Option<vk::Extent2D>,

    pub render_pass: ManuallyDrop<RenderPass>,
    pub framebuffers: ManuallyDrop<Vec<Framebuffer>>,

    pub draw_command_buffers: CommandBuffers,

    pub image_available_semaphores: Vec<vk::Semaphore>,
    pub rendering_complete_semaphores: Vec<vk::Semaphore>,
    pub frames_in_flight_fences: Vec<vk::Fence>,

    pub pipelines: ManuallyDrop<HashMap<PipelineID, Box<dyn VulkanPipelineDyn>>>,

    pub current_frame: usize,
}


impl VulkanRenderBackend {
    pub unsafe fn recreate_swapchain(self: &mut Self, surface_size: vk::Extent2D) {
        unsafe {
            self.device.device.device_wait_idle().unwrap();
            ManuallyDrop::drop(&mut self.swapchain);
            ManuallyDrop::drop(&mut self.framebuffers);
        }

        self.physical_surface.update_resolution(surface_size);
        

        self.swapchain = ManuallyDrop::new(Swapchain::new(
            &self.instance, &self.physical_surface, &self.device));

        let mut framebuffers = Vec::new();
        for &swapchain_image_view in self.swapchain.image_views.iter() {
            let framebuffer = Framebuffer::new(&self.device, &self.render_pass,
                swapchain_image_view, self.physical_surface.resolution());
            framebuffers.push(framebuffer);
        }
        self.framebuffers = ManuallyDrop::new(framebuffers);
    }
}

impl VulkanRenderBackend {
    pub fn new(window: &Arc<dyn Window>) -> Self {
        let instance =
            ManuallyDrop::new(Instance::new(window));

        let physical_surface =
            ManuallyDrop::new(PhysicalSurface::new(&instance, window));

        let device =
            ManuallyDrop::new(Arc::new(Device::new(&instance, &physical_surface)));

        let render_pass = ManuallyDrop::new(
            RenderPass::new(&physical_surface, &device));

        let swapchain = ManuallyDrop::new(Swapchain::new(
            &instance, &physical_surface, &device
        ));

        let draw_command_buffers =
            device.spawn_command_buffers(swapchain.images_count().try_into().unwrap());
        
        let semaphore_create_info = vk::SemaphoreCreateInfo::default();
        let fence_create_info = vk::FenceCreateInfo::default()
            .flags(vk::FenceCreateFlags::SIGNALED);

        let mut frames_in_flight_fences = Vec::new();
        let mut image_available_semaphores = Vec::new();
        let mut rendering_complete_semaphores = Vec::new();

        unsafe {
            for _ in 0..swapchain.images_count() {
                let fence = device.device.create_fence(&fence_create_info, None).unwrap();
                frames_in_flight_fences.push(fence);
                let image_available_semaphore = device.device.create_semaphore(&semaphore_create_info, None).unwrap();
                let rendering_complete_semaphore = device.device.create_semaphore(&semaphore_create_info, None).unwrap();
                image_available_semaphores.push(image_available_semaphore);
                rendering_complete_semaphores.push(rendering_complete_semaphore);
            }
        }

        let mut framebuffers = Vec::new();
        for &swapchain_image_view in swapchain.image_views.iter() {
            let framebuffer =
                Framebuffer::new(
                    &device,
                    &render_pass,
                    swapchain_image_view,
                    physical_surface.resolution());
            framebuffers.push(framebuffer);
        }

        let framebuffers = ManuallyDrop::new(framebuffers);

        

        // let refl = linked_program.get_reflection();
        //
        // println!("there are {} entry points in shader", refl.entry_point_reflections.len());
        // for entry in refl.entry_point_reflections {
        //     println!("{:?} shader {}(), with fields", entry.stage, entry.name);
        //     for s in entry.struct_reflections {
        //         println!("\t{}", s);
        //     }
        // }



        let pipelines = HashMap::new();
        
        Self {
            instance,
            physical_surface,
            device,
            swapchain,
            new_swapchain_size: None,

            render_pass,
            framebuffers,

            draw_command_buffers,

            image_available_semaphores,
            rendering_complete_semaphores,
            frames_in_flight_fences,

            pipelines: ManuallyDrop::new(pipelines),
            
            current_frame: 0,
        }
    }
}

impl RenderBackend for VulkanRenderBackend {
    const PERSPECTIVE_ALIGNMENT: [f32; 3] = [1f32, -1f32, -1f32];

    fn init(backend_clone: Arc<Mutex<Option<Self>>>, window: Arc<dyn Window>) where Self: Sized
    {
        let render_backend = VulkanRenderBackend::new(&window);
        backend_clone.lock().replace(render_backend);
    }

    fn render(self: &mut Self) {
        unsafe {
            self.device.device.wait_for_fences(
                &[self.frames_in_flight_fences[self.current_frame]],
                true, u64::MAX).unwrap();
        }

        if let Some(extent) = self.new_swapchain_size {
            if extent.width <= 0 || extent.height <= 0 {
                return;
            }
            unsafe { self.recreate_swapchain(extent)};
            self.new_swapchain_size = None;
        }

        let viewports = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: self.physical_surface.resolution().width as f32,
            height: self.physical_surface.resolution().height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];
        let scissors = [self.physical_surface.resolution().into()];

        unsafe {self.device.device.reset_fences(&[self.frames_in_flight_fences[self.current_frame]]).unwrap()};

        let acquire_result = unsafe {self
            .swapchain.swapchain_loader
            .acquire_next_image(
                self.swapchain.swapchain,
                u64::MAX,
                self.image_available_semaphores[self.current_frame],
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
            .render_pass(self.render_pass.render_pass)
            .framebuffer(self.framebuffers[present_index as usize].framebuffer)
            .render_area(self.physical_surface.resolution().into())
            .clear_values(&clear_values);

        unsafe {
            let device = &self.device.device;

            let draw_command_buffer =
                self.draw_command_buffers.command_buffers[self.current_frame];

            device.reset_command_buffer(
                draw_command_buffer,
                vk::CommandBufferResetFlags::RELEASE_RESOURCES
            ).expect("Failed to reset draw command buffer");

            let command_buffer_begin_info =
                vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

            device.begin_command_buffer(
                draw_command_buffer,
                &command_buffer_begin_info
            ).expect("Begin command buffer");
            
            device.cmd_begin_render_pass(
                draw_command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            );

            for (id, pipeline) in self.pipelines.iter() {
                device.cmd_bind_pipeline(
                    draw_command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline.get_pipeline(),
                );
                device.cmd_set_viewport(draw_command_buffer, 0, &viewports);
                device.cmd_set_scissor(draw_command_buffer, 0, &scissors);
                for x in pipeline.get_instances() {
                    x.draw(
                        draw_command_buffer,
                    )
                }
            }
            device.cmd_end_render_pass(draw_command_buffer);
            device.end_command_buffer(
                draw_command_buffer
            ).expect("Failed to end draw command buffer");

            let wait_semaphores = [self.image_available_semaphores[self.current_frame]];
            let command_buffers = vec![draw_command_buffer];
            let signal_semaphores = [self.rendering_complete_semaphores[present_index as usize]];

            let submit_info = vk::SubmitInfo::default()
                .wait_semaphores(&wait_semaphores)
                .wait_dst_stage_mask(&[PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
                .command_buffers(&command_buffers)
                .signal_semaphores(&signal_semaphores);

            device.queue_submit(
                self.device.present_queue,
                &[submit_info],
                self.frames_in_flight_fences[self.current_frame]
            ).expect("Drawing queue submit failed.");
        }

        unsafe {
            let signal_semaphores = [self.rendering_complete_semaphores[present_index as usize]];
            let swapchains = [self.swapchain.swapchain];
            let image_indices = [present_index];
            let present_info = vk::PresentInfoKHR::default()
                .wait_semaphores(&signal_semaphores)
                .swapchains(&swapchains)
                .image_indices(&image_indices);

            self.swapchain.swapchain_loader
                .queue_present(self.device.present_queue, &present_info)
                .unwrap()
        };
        
        self.current_frame += 1;
        self.current_frame = self.current_frame % 3;
    }

    fn process_event(self: &mut Self, _event: &WindowEvent) {
    }

    fn resize(self: &mut Self, width: u32, height: u32) {
        self.new_swapchain_size = Some(vk::Extent2D { width, height });
    }
}


impl CreateDrawletVulkan for VulkanRenderBackend
{
    fn create_pipeline<RenObjType: VulkanRenderObject>(self: &mut Self, shader_path: &str, _triangle_shader: &str) -> PipelineHandle<RenObjType> {
        let compiler = slang_refl::Compiler::new_spirv_compiler();
        let linked_program = compiler.linked_program_from_file(shader_path);

        let compiled_triangle_shader = linked_program.get_u32();

        let pipeline = RenObjType::Pipeline::new(
            &*self.device, &*self.render_pass, compiled_triangle_shader,
            self.physical_surface.resolution(), self.framebuffers.len());

        let pipeline_id: PipelineID = <Self as CreateDrawletVulkan>::get_pipeline_id();
        
        let ret = PipelineHandle::<RenObjType> {
            id: pipeline_id,
            _pipeline_ty: PhantomData::default(),
        };
        

        self.pipelines.insert(pipeline_id.clone(), Box::new(pipeline));

        ret
    }

    fn create_drawlet<RenObjType: VulkanRenderObject>(self: &mut Self, pipeline_handle: &PipelineHandle<RenObjType>, init_data: <<RenObjType as VulkanRenderObject>::Drawlet as RenderDrawlet>::Data) -> DrawletHandle<RenObjType>
    {
        let pipeline= self.pipelines.get_mut(&pipeline_handle.id).unwrap();
        let pipeline_any = pipeline.as_any_mut();
        let pipeline_concrete = pipeline_any.downcast_mut::<RenObjType::Pipeline>().unwrap();
        
        pipeline_concrete.instantiate_drawlet(init_data)
    }

    fn get_drawlet_mut<RenObjType: VulkanRenderObject>(self: &mut Self, pipeline_handle: &PipelineHandle<RenObjType>, drawlet_handle: &DrawletHandle<RenObjType>) -> &'_ mut RenObjType::Drawlet {
        let pipeline= self.pipelines.get_mut(&pipeline_handle.id).unwrap();
        let pipeline_any = pipeline.as_any_mut();
        let pipeline_concrete = pipeline_any.downcast_mut::<RenObjType::Pipeline>().unwrap();
        
        pipeline_concrete.get_drawlet_mut(&drawlet_handle)
    }
}

impl Drop for VulkanRenderBackend {
    fn drop(&mut self) {
        println!("Dropping application.");
        unsafe {
            self.device.device.device_wait_idle().unwrap();

            for i in 0..self.framebuffers.len() {
                self.device.device.destroy_semaphore(self.image_available_semaphores[i], None);
                self.device.device.destroy_semaphore(self.rendering_complete_semaphores[i], None);

                self.device.device.destroy_fence(self.frames_in_flight_fences[i], None);
            }

            ManuallyDrop::drop(&mut self.framebuffers);

            ManuallyDrop::drop(&mut self.pipelines);
            ManuallyDrop::drop(&mut self.render_pass);
            ManuallyDrop::drop(&mut self.swapchain);

            ManuallyDrop::drop(&mut self.device);
            ManuallyDrop::drop(&mut self.physical_surface);
            ManuallyDrop::drop(&mut self.instance);
        }
    }
}

pub trait CreateDrawletVulkan
{
    fn create_pipeline<RenObjType: VulkanRenderObject>
    (
        self: &mut Self,
        shader_path: &str,
        triangle_shader: &str
    ) -> PipelineHandle<RenObjType>;

    fn create_drawlet<RenObjType: VulkanRenderObject>(
        self: &mut Self,
        pipeline: &PipelineHandle<RenObjType>,
        init_data: <RenObjType::Drawlet as RenderDrawlet>::Data,
    ) -> DrawletHandle<RenObjType>;

    fn get_drawlet_mut<RenObjType: VulkanRenderObject>(
        self: &mut Self,
        pipeline_handle: &PipelineHandle<RenObjType>,
        drawlet_handle: &DrawletHandle<RenObjType>
    ) -> &'_ mut RenObjType::Drawlet;

    fn get_pipeline_id() -> PipelineID {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER:AtomicUsize = AtomicUsize::new(1);
        PipelineID(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}
