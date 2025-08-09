use std::any::Any;
use std::marker::PhantomData;
use std::sync::{Arc, Weak};
use ash::vk;
use ash::vk::CommandBuffer;
use bytemuck::{Pod, Zeroable};
use image::DynamicImage;
use winit::window::Window;
use parking_lot::Mutex;
use wgpu::SurfaceConfiguration;
use wgpu::wgc::command::DrawError;
use winit::event::WindowEvent;
use crate::render_backend::vulkan::device::Device;
use crate::render_backend::vulkan::render_object::TexturedMeshDrawlet as VkTexturedMeshDrawlet;
use crate::render_backend::vulkan::render_object::TexturedMeshPipeline as VkTexturedMeshPipeline;
use crate::render_backend::web::textured_mesh::TexturedMeshDrawlet as WgpuTexturedMeshDrawlet;
use crate::render_backend::web::textured_mesh::TexturedMeshPipeline as WgpuTexturedMeshPipeline;
use crate::render_backend::vulkan::render_pass::RenderPass;

pub mod vulkan;
pub mod web;

#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub tex_coord: [f32; 2]
}

#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub struct Mat4Ubo {
    pub mvp: cgmath::Matrix4<f32>
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

pub trait RenderPipeline<RenObj: RenderObject> {
    fn get_drawlet_id() -> DrawletID {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER:AtomicUsize = AtomicUsize::new(1);
        DrawletID(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

pub trait RenderDrawlet: Sized {
    type Data;
}

pub struct PipelineHandle<D:RenderObject> {
    id: PipelineID,
    _pipeline_ty: PhantomData<D>
}

pub struct DrawletHandle<D:RenderObject> {
    id: DrawletID,
    _drawlet_ty: PhantomData<D>
}

pub trait VulkanPipelineDyn {
    fn get_pipeline(self: &Self) -> vk::Pipeline;
    fn get_instances(self: &Self) -> Box<dyn Iterator<Item=&dyn VulkanDrawletDyn> + '_>;
    fn get_instances_mut(self: &mut Self) -> Box<dyn Iterator<Item=&mut dyn VulkanDrawletDyn> + '_>;
    fn as_any_mut(self: &mut Self) -> &mut dyn Any;
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

pub trait WgpuPipeline<RenObjType: WgpuRenderObject>: RenderPipeline<RenObjType> + WgpuPipelineDyn {
    fn instantiate_drawlet(
        self: &mut Self,
        init_data: <<RenObjType as WgpuRenderObject>::Drawlet as RenderDrawlet>::Data
    ) -> DrawletHandle<RenObjType>;

    fn get_drawlet_mut(self: &mut Self, drawlet_handle: &DrawletHandle<RenObjType>) -> &'_ mut RenObjType::Drawlet ;
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

pub trait CreateDrawletVulkan
{
    fn create_pipeline<RenObjType: VulkanRenderObject>
    (
        self: &mut Self,
        shader_path: &str
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

pub trait CreateDrawletWgpu
{
    fn create_pipeline<RenObjType: WgpuRenderObject>(
        self: &mut Self,
        shader_path: &str
    ) -> PipelineHandle<RenObjType>;

    fn create_drawlet<RenObjType: WgpuRenderObject>(
        self: &mut Self,
        pipeline: &PipelineHandle<RenObjType>,
        init_data: <RenObjType::Drawlet as RenderDrawlet>::Data,
    ) -> DrawletHandle<RenObjType>;

    fn get_drawlet_mut<RenObjType: WgpuRenderObject>(
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

pub struct TexturedMeshData {
    pub index_data: Vec<u32>,
    pub vertex_data: Vec<Vertex>,
    pub texture_data: DynamicImage
}

pub struct TexturedMesh {}

pub trait RenderObject {}
pub trait VulkanRenderObject: RenderObject + Sized {
    type Drawlet: VulkanDrawlet;
    type Pipeline: VulkanPipeline<Self> + VulkanPipelineDyn + 'static;
    type Data;
    
}
impl RenderObject for TexturedMesh {}
impl VulkanRenderObject for TexturedMesh {
    type Drawlet = VkTexturedMeshDrawlet;
    type Pipeline = VkTexturedMeshPipeline;
    type Data = TexturedMeshData;
}

pub trait WgpuRenderObject: RenderObject + Sized {
    type Drawlet: WgpuDrawlet;
    type Pipeline: WgpuPipeline<Self> + WgpuPipelineDyn + 'static;
    type Data;
}

impl WgpuRenderObject for TexturedMesh {
    type Drawlet = WgpuTexturedMeshDrawlet;
    type Pipeline = WgpuTexturedMeshPipeline;
    type Data = TexturedMeshData;
}