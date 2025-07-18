use ash::vk;
use crate::render_backend::vulkan::Instance;
use crate::render_backend::vulkan::physical_surface::PhysicalSurface;
use ash::khr::{swapchain as ash_swapchain};
use crate::render_backend::vulkan::command_buffer::{CommandBuffers};

pub struct Device {
    pub device: ash::Device,
    pub present_queue: vk::Queue,
    pub command_pool: vk::CommandPool,
    pub physical_memory_properties: vk::PhysicalDeviceMemoryProperties,
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

        let pool_create_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(physical_surface.queue_family_index);

        let command_pool = unsafe {device.create_command_pool(&pool_create_info, None).unwrap()};

        let memory_properties = unsafe {
            instance.instance.get_physical_device_memory_properties(
                physical_surface.physical_device)
        };

        Self {
            device,
            present_queue,
            command_pool,
            physical_memory_properties: memory_properties
        }
    }

    pub fn spawn_command_buffers(self: &Self, count: u32) -> CommandBuffers{
        return CommandBuffers::new(self, count);
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_command_pool(self.command_pool, None);
            self.device.destroy_device(None);
        }
    }
}