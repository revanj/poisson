use std::sync;
use std::sync::Arc;
use ash::vk;
use image::{RgbImage, RgbaImage};
use crate::render_backend::vulkan::buffer::GpuBuffer;
use crate::render_backend::vulkan::device::Device;
use crate::render_backend::vulkan::img;
use crate::render_backend::vulkan::img::Image;

pub struct Texture {
    device: sync::Weak<Device>,
    pub image: Image,
    pub sampler: vk::Sampler,
}

impl Texture {
    pub fn from_image(device: &Arc<Device>, img: &RgbaImage) -> Self {
        let image_extents = img.dimensions();
        let image_raw = img.as_raw();
        let image_slice = image_raw.as_slice();

        let mut texture_buffer = GpuBuffer::<u8>::create_buffer(
            &device,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
            (image_extents.0 * image_extents.1 * 4) as usize);
        texture_buffer.map();
        texture_buffer.write(image_slice);
        texture_buffer.unmap();

        let texture_image = img::Image::create_image(
            &device,
            &texture_buffer.buffer,
            vk::Format::R8G8B8A8_SRGB,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            vk::Extent2D { width: image_extents.0, height: image_extents.1 }
        );

        //drop(texture_buffer);

        let sampler_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::REPEAT)
            .address_mode_v(vk::SamplerAddressMode::REPEAT)
            .address_mode_w(vk::SamplerAddressMode::REPEAT)
            .anisotropy_enable(true)
            .max_anisotropy(device.physical_device_properties.limits.max_sampler_anisotropy)
            .border_color(vk::BorderColor::INT_TRANSPARENT_BLACK)
            .unnormalized_coordinates(false)
            .compare_enable(false)
            .compare_op(vk::CompareOp::ALWAYS)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .mip_lod_bias(0.0)
            .min_lod(0.0)
            .max_lod(0.0);

        let sampler = unsafe {
            device.device.create_sampler(&sampler_info, None)
        }.unwrap();

        Self {
            device: Arc::downgrade(device),
            image: texture_image,
            sampler
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        let device = self.device.upgrade().unwrap();
        unsafe {
            device.device.destroy_sampler(self.sampler, None);
        }
    }
}