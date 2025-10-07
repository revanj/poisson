use std::ptr::slice_from_raw_parts;
use cgmath::Matrix;
use wgpu::{BindGroup, BindGroupLayout, Device};
use wgpu::util::DeviceExt;
use crate::render_backend::web::gpu_resources::interface::WgpuUniformResource;

pub struct GpuMat4 {
    pub buffer: wgpu::Buffer,
    bind_group: BindGroup,
}

impl GpuMat4 {
    pub fn from_mat4(device: &Device, mat4: &cgmath::Matrix4<f32>) -> Self {
        let uniform_ptr = mat4.as_ptr();
        let uniform_slice = unsafe {
            &*slice_from_raw_parts(uniform_ptr as *const u8, size_of::<cgmath::Matrix4<f32>>())
        };

        let buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(uniform_slice),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let bind_group_layout = Self::create_bind_group_layout(device);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }
            ],
            label: Some("camera_bind_group"),
        });

        Self { buffer, bind_group }
    }
}

impl WgpuUniformResource for GpuMat4 {
    fn create_bind_group_layout(device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("GpuMat4 Bind Group Layout"),
        })
    }

    fn get_bind_group(self: &Self) -> &BindGroup {
        &self.bind_group
    }
}