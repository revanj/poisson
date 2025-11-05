use std::ptr::slice_from_raw_parts;
use cgmath::{Array, Matrix};
use wgpu::{BindGroup, BindGroupLayout, Device};
use wgpu::util::DeviceExt;
use crate::render_backend::web::gpu_resources::interface::WgpuUniformResource;

pub struct GpuVec4 {
    pub buffer: wgpu::Buffer,
    bind_group: BindGroup,
}

impl GpuVec4 {
    pub fn from_vec4(device: &Device, vec4: &cgmath::Vector4<f32>) -> Self {
        let uniform_ptr = vec4.as_ptr();
        let uniform_slice = unsafe {
            &*slice_from_raw_parts(uniform_ptr as *const u8, size_of::<cgmath::Vector4<f32>>())
        };

        let buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Light Dir Buffer"),
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
            label: Some("light_dir_bind_group"),
        });

        Self { buffer, bind_group }
    }
}

impl WgpuUniformResource for GpuVec4 {
    fn create_bind_group_layout(device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("GpuVec4 Bind Group Layout"),
        })
    }

    fn get_bind_group(self: &Self) -> &BindGroup {
        &self.bind_group
    }
}