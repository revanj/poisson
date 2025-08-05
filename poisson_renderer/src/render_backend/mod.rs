use std::any::Any;
use std::marker::PhantomData;
use std::sync::{Arc, Weak};
use ash::vk;
use ash::vk::CommandBuffer;
use winit::window::Window;
use parking_lot::Mutex;
use winit::event::WindowEvent;
use crate::render_backend::vulkan::device::Device;
use crate::render_backend::vulkan::render_pass::RenderPass;

pub mod vulkan;
pub mod web;


#[derive(Clone, Debug, Copy)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub color: [f32; 3],
    pub tex_coord: [f32; 2]
}

#[derive(Clone, Debug, Copy)]
pub struct UniformBufferObject {
    pub model: cgmath::Matrix4<f32>,
    pub view: cgmath::Matrix4<f32>,
    pub proj: cgmath::Matrix4<f32>,
}

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct PipelineID(usize);

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct DrawletID(usize);


// pub struct Renderer<BackendImpl: RenderBackend> {
//     render_loop: BackendImpl
// }
//
// impl<BackendImpl: RenderBackend> Renderer<BackendImpl>
// {
//     pub fn render(self: &mut Self) {
//         self.render_loop.render();
//     }
//     pub fn create_pipeline<PipelineType: RenderPipeline>(self: &mut Self, shader_path: &str) -> PipelineHandle<PipelineType>
//         where Renderer<BackendImpl>: CreatePipeline<PipelineType>,
//     {
//         CreatePipeline::create_pipeline(self, &shader_path)
//     }
// }

pub trait RenderBackend {
    fn init(backend_clone: Arc<Mutex<Option<Self>>>, window: Arc<dyn Window>) where Self: Sized;
    fn render(self: &mut Self);
    fn process_event(self: &mut Self, event: &WindowEvent);
    fn resize(self: &mut Self, width: u32, height: u32);
}

pub trait RenderPipeline {
    type DrawletType: RenderDrawlet;
    type DrawletDataType;
    
    fn get_drawlet_id() -> DrawletID {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER:AtomicUsize = AtomicUsize::new(1);
        DrawletID(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

pub trait RenderDrawlet {}
pub struct PipelineHandle<P:RenderPipeline> {
    id: PipelineID,
    _pipeline_ty: PhantomData<P>
}

pub struct DrawletHandle<D:RenderDrawlet> {
    id: DrawletID,
    _drawlet_ty: PhantomData<D>
}

pub trait VulkanPipelineObj: VulkanPipelineDyn + VulkanPipeline {}
pub trait VulkanPipelineDyn {
    fn get_pipeline(self: &Self) -> vk::Pipeline;
    fn get_instances(self: &Self) -> Box<dyn Iterator<Item=&dyn VulkanDrawlet> + '_>;
    fn get_instances_mut(self: &mut Self) -> Box<dyn Iterator<Item=&mut dyn VulkanDrawlet> + '_>;

    fn as_any_mut(self: &mut Self) -> &mut dyn Any;
}

pub trait VulkanPipeline: RenderPipeline {
    fn new(device: &Arc<Device>,
           render_pass: &RenderPass,
           shader_bytecode: &[u32],
           resolution: vk::Extent2D,
           n_framebuffers: usize,
    ) -> Self where Self: Sized;
    fn instantiate_drawlet(
        self: &mut Self,
        init_data: Self::DrawletDataType
    ) -> DrawletHandle<Self::DrawletType>;
}

pub trait WgpuPipeline: RenderPipeline {}

pub trait VulkanDrawlet: RenderDrawlet {
    fn draw(self: &Self, command_buffer: CommandBuffer, current_frame: usize);
    fn update_uniform_buffer(self: &mut Self, current_frame: usize, elapsed_time: f32);
}
pub trait WgpuDrawlet: RenderDrawlet {}

pub trait CreatePipeline<RenderPipelineType: RenderPipeline>
{
    fn create_pipeline(
        self: &mut Self,
        shader_path: &str
    ) -> PipelineHandle<RenderPipelineType>;

    fn create_drawlet(
        self: &mut Self,
        pipeline: &PipelineHandle<RenderPipelineType>,
        init_data: RenderPipelineType::DrawletDataType,
    ) -> DrawletHandle<RenderPipelineType::DrawletType>;

    fn get_pipeline_id() -> PipelineID {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER:AtomicUsize = AtomicUsize::new(1);
        PipelineID(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}
