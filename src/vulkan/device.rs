use ash::vk;
use crate::vulkan::Instance;
use crate::vulkan::physical_surface::PhysicalSurface;
use ash::khr::{swapchain as ash_swapchain};

pub struct Device {
    pub device: ash::Device,
    pub present_queue: vk::Queue,
}

impl Device {
    pub fn new(instance: &Instance, physical_surface: &PhysicalSurface) -> Self {
        let device_extension_names_raw = [
            ash_swapchain::NAME.as_ptr(),
        ];

        let features = vk::PhysicalDeviceFeatures {
            shader_clip_distance: 1,
            ..Default::default()
        };
        let priorities = [1.0];

        let queue_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(physical_surface.queue_family_index)
            .queue_priorities(&priorities);

        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(std::slice::from_ref(&queue_info))
            .enabled_extension_names(&device_extension_names_raw)
            .enabled_features(&features);
        
        
        let device: ash::Device = unsafe {
            instance.instance
                .create_device(physical_surface.physical_device, &device_create_info, None)
                .unwrap()
        };
        
        let present_queue = unsafe {
            device.get_device_queue(physical_surface.queue_family_index, 0)
        };
        
        Self {
            device,
            present_queue
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_device(None);
        }
    }
}