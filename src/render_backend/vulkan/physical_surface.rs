use ash::khr::surface;
use ash::vk;
use ash::vk::{PresentModeKHR, SurfaceTransformFlagsKHR};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::Window;

use crate::render_backend::vulkan;
use vulkan::Destroy;

pub struct PhysicalSurface {
    pub surface: vk::SurfaceKHR,
    pub surface_loader: surface::Instance,
    pub physical_device: vk::PhysicalDevice,
    pub queue_family_index: u32,
    pub surface_format: vk::SurfaceFormatKHR,
    pub surface_capabilities: vk::SurfaceCapabilitiesKHR,
    pub surface_resolution: vk::Extent2D

}

impl PhysicalSurface {
    pub fn new(instance: &vulkan::Instance, window: &Box<dyn Window>) -> Self {
        let surface = unsafe {
            ash_window::create_surface(
                &instance.entry, &instance.instance,
                window.display_handle().unwrap().as_raw(),
                window.window_handle().unwrap().as_raw(),
                None).unwrap()
        };

        let physical_devices = unsafe {
            instance.instance.enumerate_physical_devices()
                .expect("Failed to enumerate physical devices")
        };
        
        let surface_loader = surface::Instance::new(&instance.entry, &instance.instance);
        let mut selected_device = None;

        for physical_device in physical_devices.iter() {
            let queue_family_properties =
                unsafe {
                    instance.instance
                    .get_physical_device_queue_family_properties(*physical_device)
                };

            for (index, info) in queue_family_properties.iter().enumerate() {
                let supports_graphics = info.queue_flags.contains(vk::QueueFlags::GRAPHICS);
                let supports_surface = unsafe {
                    surface_loader
                    .get_physical_device_surface_support(
                        *physical_device, index as u32, surface
                    )
                    .unwrap() 
                };

                if supports_graphics && supports_surface {
                    selected_device = Some((*physical_device, index));
                    break;
                }
            }

            if selected_device.is_some() {
                break;
            }
        };
        
        let (physical_device, queue_family_index) = selected_device
            .expect("Failed to find suitable physical device");

        let queue_family_index = queue_family_index as u32;
        
        let surface_format = unsafe { surface_loader
            .get_physical_device_surface_formats(physical_device, surface)
            .unwrap()[0] }; 

        let surface_capabilities = unsafe { surface_loader
            .get_physical_device_surface_capabilities(physical_device, surface)
            .unwrap() };

        let window_size = window.surface_size();
        let window_extent = vk::Extent2D {
            width: window_size.width,
            height: window_size.height,
        };

        let surface_resolution =
            match surface_capabilities.current_extent.width {
                u32::MAX => window_extent,
                _ => surface_capabilities.current_extent,
            };

        Self {
            surface,
            surface_loader,
            physical_device,
            queue_family_index,
            surface_format,
            surface_capabilities,
            surface_resolution
        }
    }

    pub fn update_resolution(&mut self, window_extent: vk::Extent2D) {
        self.surface_capabilities = unsafe { self.surface_loader
            .get_physical_device_surface_capabilities(self.physical_device, self.surface)
            .unwrap()
        };

        self.surface_resolution =
            match self.surface_capabilities.current_extent.width {
                u32::MAX => window_extent,
                _ => self.surface_capabilities.current_extent,
            };
    }

    pub fn resolution(self: &Self) -> vk::Extent2D {
        self.surface_resolution
    }

    pub fn swapchain_image_count(self: &Self) -> u32 {
        let mut desired_image_count = self.surface_capabilities.min_image_count + 1;
        if self.surface_capabilities.max_image_count > 0
            && desired_image_count > self.surface_capabilities.max_image_count
        {
            desired_image_count = self.surface_capabilities.max_image_count;
        }

        desired_image_count
    }


    pub fn pre_transform(self: &Self) -> SurfaceTransformFlagsKHR {
        let pre_transform = if self.surface_capabilities
            .supported_transforms
            .contains(SurfaceTransformFlagsKHR::IDENTITY)
        {
            SurfaceTransformFlagsKHR::IDENTITY
        } else {
            self.surface_capabilities.current_transform
        };

        pre_transform
    }

    pub fn present_mode(self: &Self) -> PresentModeKHR {
        let present_modes = unsafe {
            self.surface_loader
            .get_physical_device_surface_present_modes(
                self.physical_device,
                self.surface)
            .unwrap() };
        let present_mode = present_modes
            .iter()
            .cloned()
            .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
            .unwrap_or(vk::PresentModeKHR::FIFO);

        present_mode
    }

}

impl Destroy for PhysicalSurface {
    fn destroy(&mut self) {
        unsafe {
            self.surface_loader.destroy_surface(self.surface, None);
        }
    }
}