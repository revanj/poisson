use std::marker::PhantomData;

fn main() {
    println!("Hello, world!");
    
}

trait Ren {
    type Buffer: ;
}
struct Wgpu {} impl Ren for Wgpu {
    type Buffer = ();
}
struct Vk {} impl Ren for Vk {
    type Buffer = ();
}

trait Backend<R: Ren> {
    fn create_texture(self: &Self) -> impl Texture<R>;
    fn create_buffer(self: &Self) -> impl Buffer<R>;
    fn create_render_pass(self: &Self) -> impl RenderPass<R>;
    fn create_graphics_pipeline(self: &Self) -> impl GraphicsPipeline<R>;
    fn create_compute_pipeline(self: &Self) -> impl ComputePipeline<R>;
    fn create_drawlet<D: Drawlet<R>>(self: &Self) -> D::Instance;
}

struct VkBackend {
    
}

impl Backend<Vk> for VkBackend {
    fn create_texture(self: &Self) -> impl Texture<Vk> {
        todo!()
    }

    fn create_buffer(self: &Self) -> impl Buffer<Vk> {
        todo!()
    }

    fn create_render_pass(self: &Self) -> impl RenderPass<Vk> {
        todo!()
    }

    fn create_graphics_pipeline(self: &Self) -> impl GraphicsPipeline<Vk> {
        todo!()
    }

    fn create_compute_pipeline(self: &Self) -> impl ComputePipeline<Vk> {
        todo!()
    }

    fn create_drawlet<D: Drawlet<Vk>>(self: &Self) -> D::Instance {
        todo!()
    }
}

trait RenderPass<R> {}
trait GraphicsPipeline<R> {}
trait ComputePipeline<R> {}
trait Buffer<R> {}

trait DrawletData<R> {
    
}
trait Drawlet<R> {
    type Data: DrawletData<R>;
    type Instance: DrawletInstance<R>;
}
trait DrawletInstance<R> {
    
}


struct WgpuTexturedMesh {
    
} impl Drawlet<Wgpu> for WgpuTexturedMesh {
    type Data = WgpuTexturedMeshData;
}
struct WgpuTexturedMeshData {
    buffer: <Wgpu as Ren>::Buffer,
} impl DrawletData<Wgpu> for WgpuTexturedMeshData {}



trait Texture<R> {}
