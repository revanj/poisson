use ash::vk;

pub struct PhysicalDevice {
    physical_device: vk::PhysicalDevice,
    queue_family_index: u32,
}