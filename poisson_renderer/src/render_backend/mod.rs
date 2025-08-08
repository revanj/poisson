use std::any::Any;
use std::marker::PhantomData;
use std::sync::{Arc, Weak};
use ash::vk;
use ash::vk::CommandBuffer;
use image::DynamicImage;
use winit::window::Window;
use parking_lot::Mutex;
use wgpu::SurfaceConfiguration;
use winit::event::WindowEvent;
use crate::render_backend::vulkan::device::Device;
use crate::render_backend::vulkan::render_pass::RenderPass;

pub mod vulkan;
pub mod web;

#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub struct Vertex {
    pub pos: [f32; 3],
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


pub trait RenderBackend {
    // Common Drawlet Types
    type TexturedMesh: RenderDrawlet;
    
    fn init(backend_clone: Arc<Mutex<Option<Self>>>, window: Arc<dyn Window>) where Self: Sized;
    fn render(self: &mut Self);
    fn process_event(self: &mut Self, event: &WindowEvent);
    fn resize(self: &mut Self, width: u32, height: u32);
}

pub trait RenderPipeline<DrawletType: RenderDrawlet> {
    fn instantiate_drawlet(
        self: &mut Self,
        init_data: DrawletType::Data
    ) -> DrawletHandle<DrawletType>;

    fn get_drawlet_mut(self: &mut Self, drawlet_handle: &DrawletHandle<DrawletType>) -> &'_ mut DrawletType;

    fn get_drawlet_id() -> DrawletID {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER:AtomicUsize = AtomicUsize::new(1);
        DrawletID(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

pub trait RenderDrawlet: Sized {
    type Pipeline: RenderPipeline<Self>;
    type Data;
}

pub struct PipelineHandle<D:RenderDrawlet> {
    id: PipelineID,
    _pipeline_ty: PhantomData<D>
}

pub struct DrawletHandle<D:RenderDrawlet> {
    id: DrawletID,
    _drawlet_ty: PhantomData<D>
}

pub trait VulkanPipelineDyn {
    fn get_pipeline(self: &Self) -> vk::Pipeline;
    fn get_instances(self: &Self) -> Box<dyn Iterator<Item=&dyn VulkanDrawletDyn> + '_>;
    fn get_instances_mut(self: &mut Self) -> Box<dyn Iterator<Item=&mut dyn VulkanDrawletDyn> + '_>;
    fn as_any_mut(self: &mut Self) -> &mut dyn Any;
}

pub trait VulkanPipeline<DrawletType: VulkanDrawlet>: RenderPipeline<DrawletType> + VulkanPipelineDyn {
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

pub trait WgpuPipeline<DrawletType: WgpuDrawlet>: RenderPipeline<DrawletType> + WgpuPipelineDyn {
    fn new(
        device: &Arc<wgpu::Device>,
        queue: &Arc<wgpu::Queue>,
        shader_path: &str,
        surface_config: &SurfaceConfiguration
    ) -> Self where Self: Sized;
}

pub trait WgpuPipelineDyn {
    fn get_pipeline(self: &Self) -> &wgpu::RenderPipeline;
    fn get_instances(self: &Self) -> Box<dyn Iterator<Item=&dyn WgpuDrawletDyn> + '_>;
    fn get_instances_mut(self: &mut Self) -> Box<dyn Iterator<Item=&mut dyn WgpuDrawletDyn> + '_>;
    fn as_any_mut(self: &mut Self) -> &mut dyn Any;
}
pub trait WgpuDrawlet: RenderDrawlet {
    fn draw(self: &Self, render_pass: &mut wgpu::RenderPass);
}
pub trait WgpuDrawletDyn {
    fn draw(self: &Self, render_pass: &mut wgpu::RenderPass);
}
impl<T> WgpuDrawletDyn for T where T: WgpuDrawlet {
    fn draw(self: &Self, render_pass: &mut wgpu::RenderPass) {
        self.draw(render_pass);
    }
}

pub trait CreateDrawlet<DrawletType: RenderDrawlet>
{
    fn create_pipeline(
        self: &mut Self,
        shader_path: &str
    ) -> PipelineHandle<DrawletType>;

    fn create_drawlet(
        self: &mut Self,
        pipeline: &PipelineHandle<DrawletType>,
        init_data: DrawletType::Data,
    ) -> DrawletHandle<DrawletType>;
    
    fn get_drawlet_mut(
        self: &mut Self,
        pipeline_handle: &PipelineHandle<DrawletType>,
        drawlet_handle: &DrawletHandle<DrawletType>
    ) -> &'_ mut DrawletType;

    fn get_pipeline_id() -> PipelineID {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER:AtomicUsize = AtomicUsize::new(1);
        PipelineID(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

pub struct TexturedMeshData {
    pub index_data: Vec<u32>,
    pub vertex_data: Vec<Vertex>,
    pub texture_data: DynamicImage
}