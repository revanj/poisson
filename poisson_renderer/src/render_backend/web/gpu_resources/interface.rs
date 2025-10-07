use wgpu::{BindGroup, BindGroupLayout};

pub trait WgpuUniformResource {
    fn create_bind_group_layout(device: &wgpu::Device) -> BindGroupLayout;
    fn get_bind_group(self: &Self) -> &BindGroup;
}