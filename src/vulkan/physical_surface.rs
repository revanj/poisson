use ash::khr::surface;
use ash::vk;
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::Window;
use crate::vulkan;

pub struct PhysicalSurface {
    pub surface: vk::SurfaceKHR,
    pub surface_loader: surface::Instance,
    pub physical_device: vk::PhysicalDevice,
    pub queue_family_index: u32,
    pub surface_format: vk::SurfaceFormatKHR,
    pub surface_capabilities: vk::SurfaceCapabilitiesKHR,
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
        
        Self {
            surface,
            surface_loader,
            physical_device,
            queue_family_index,
            surface_format,
            surface_capabilities
        }
        
    }
}

impl Drop for PhysicalSurface {
    fn drop(&mut self) {
        unsafe {
            println!("destroying surface");
            self.surface_loader.destroy_surface(self.surface, None);
        }
    }
}