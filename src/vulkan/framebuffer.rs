use std::ops::Deref;
use std::sync::Arc;
use ash::vk;
use ash::vk::{Extent2D, ImageView};
use crate::vulkan::render_pass::RenderPass;
use crate::vulkan::device::Device;
use crate::vulkan::image::Image;
use crate::vulkan::swapchain::Swapchain;

pub struct Framebuffer {
    pub device: std::sync::Weak<Device>,
    pub depth_image: Image,
    pub framebuffer: vk::Framebuffer
}

impl Framebuffer {
    pub fn new(
        dev: &Arc<Device>,
        render_pass: &RenderPass,
        swapchain_view: ImageView,
        resolution: Extent2D) -> Self
    {
        let device = Arc::downgrade(dev);
        let depth_image = Image::new_depth_image(dev, resolution);
        let framebuffer_attachments = [swapchain_view, depth_image.view];
        let frame_buffer_create_info = vk::FramebufferCreateInfo::default()
            .render_pass(render_pass.render_pass)
            .attachments(&framebuffer_attachments)
            .width(resolution.width)
            .height(resolution.height)
            .layers(1);


        let framebuffer = unsafe {dev.device
            .create_framebuffer(&frame_buffer_create_info, None)
            .unwrap()};

        Self {
            device,
            depth_image,
            framebuffer
        }
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        let device = self.device.upgrade().unwrap();
        unsafe {
            device.device.destroy_framebuffer(self.framebuffer, None);
        }
    }
}