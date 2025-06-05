use std::sync;
use std::sync::Arc;
use ash::vk;
use ash::vk::Extent2D;
use crate::vulkan::device::Device;
use crate::vulkan::Instance;
use crate::vulkan::physical_surface::PhysicalSurface;

pub struct Swapchain {
    pub device: sync::Weak<Device>,
    pub swapchain_loader: ash::khr::swapchain::Device,
    pub swapchain: vk::SwapchainKHR,
    pub images: Vec<vk::Image>,
    pub image_views: Vec<vk::ImageView>
}

impl Swapchain {
    pub fn new(
        instance: &Instance,
        physical_surface: &PhysicalSurface,
        device: &Arc<Device>) -> Self
    {
        let device = Arc::downgrade(device);
        let dev = device.upgrade().unwrap();

        let swapchain_loader =
            ash::khr::swapchain::Device::new(
                &instance.instance,
                &dev.device);

        let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(physical_surface.surface)
            .min_image_count(physical_surface.swapchain_image_count())
            .image_color_space(physical_surface.surface_format.color_space)
            .image_format(physical_surface.surface_format.format)
            .image_extent(physical_surface.surface_resolution)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(physical_surface.pre_transform())
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(physical_surface.present_mode())
            .clipped(true)
            .image_array_layers(1);

        let swapchain = unsafe {
            swapchain_loader
                .create_swapchain(&swapchain_create_info, None)
                .unwrap() };

        let images = unsafe { swapchain_loader.get_swapchain_images(swapchain).unwrap() };
        let image_views = images.iter().map(|&image| {
            let create_view_info = vk::ImageViewCreateInfo::default()
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(physical_surface.surface_format.format)
                .components(vk::ComponentMapping {
                    r: vk::ComponentSwizzle::R,
                    g: vk::ComponentSwizzle::G,
                    b: vk::ComponentSwizzle::B,
                    a: vk::ComponentSwizzle::A,
                })
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .image(image);
            unsafe { dev.device.create_image_view(&create_view_info, None) }.unwrap()
        }).collect();

        Self {
            device,
            swapchain_loader,
            swapchain,
            images,
            image_views
        }
    }

    pub fn images_count(self: &Self) -> usize {
        self.images.len()
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            for &image_view in self.image_views.iter() {
                self.device.upgrade().unwrap()
                    .device.destroy_image_view(image_view, None);
            }
            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
        }
    }
}