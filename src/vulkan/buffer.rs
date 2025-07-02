use std::ffi::c_void;
use std::marker;
use std::marker::PhantomData;
use std::ptr::null;
use std::sync::Arc;
use ash::util::Align;
use ash::vk;
use ash::vk::{BufferUsageFlags, DeviceSize, SharingMode};
use crate::vulkan::device::Device;

pub enum BufferType {
    Uniform,
}


// a buffer on the GPU that is a list of a fixed type
pub struct GpuBuffer<T: Copy, const count: usize> {
    pub device: std::sync::Weak<Device>,
    pub buffer: vk::Buffer,
    pub allocated_size: vk::DeviceSize,
    pub memory: vk::DeviceMemory,
    pub mapped_ptr: Option<*mut T>,
}


// https://github.com/adrien-ben/vulkan-tutorial-rs/blob/3506ed0ded0cf68915b56fc3ea2a96f9aaf21f12/src/main.rs
fn find_memory_type(
    requirements: vk::MemoryRequirements,
    mem_properties: vk::PhysicalDeviceMemoryProperties,
    required_properties: vk::MemoryPropertyFlags,
) -> u32 {
    for i in 0..mem_properties.memory_type_count {
        if requirements.memory_type_bits & (1 << i) != 0
            && mem_properties.memory_types[i as usize]
            .property_flags
            .contains(required_properties)
        {
            return i;
        }
    }
    panic!("Failed to find suitable memory type.")
}


impl<T: Copy, const count: usize> GpuBuffer<T, count> {
    const ALIGNED_ELEMENT_SIZE: DeviceSize = align_of::<T>() as DeviceSize;
    fn create_mapped_uniform(device: &Arc<Device>) -> GpuBuffer<T, count> {
        let create_info = vk::BufferCreateInfo {
            size: Self::ALIGNED_ELEMENT_SIZE * (count as DeviceSize),
            usage: BufferUsageFlags::UNIFORM_BUFFER,
            sharing_mode: SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let buffer = unsafe { device.device.create_buffer(&create_info, None) }.unwrap();
        let mem_requirements = unsafe { device.device.get_buffer_memory_requirements(buffer) };

        let physical_memory_properties = device.physical_memory_properties;

        let memory_type = find_memory_type(
            mem_requirements, physical_memory_properties,
            vk::MemoryPropertyFlags::HOST_VISIBLE
                | vk::MemoryPropertyFlags::HOST_COHERENT);

        let alloc_info = vk::MemoryAllocateInfo {
            memory_type_index: memory_type,
            allocation_size: mem_requirements.size,
            ..Default::default()
        };

        let (memory, mapped_ptr) = unsafe {
            let mem = device.device.allocate_memory(&alloc_info, None).unwrap();
            device.device.bind_buffer_memory(buffer, mem, 0).unwrap();

            let ptr = device.device
                .map_memory(mem, 0, mem_requirements.size, vk::MemoryMapFlags::empty())
                .unwrap();

            (mem, ptr)
        };

        Self {
            device: Arc::downgrade(device),
            buffer,
            allocated_size: mem_requirements.size,
            memory,
            mapped_ptr: Some(mapped_ptr.cast()),
        }
    }

    fn map(self: &Self) {
        let device = self.device.upgrade().unwrap();
        unsafe {
            device.device.map_memory(
                self.memory, 0, self.allocated_size, vk::MemoryMapFlags::empty())
        }.unwrap();
    }
    
    fn unmap(self: &Self) {
        let device = self.device.upgrade().unwrap();
        unsafe {
            device.device.unmap_memory(self.memory);
        }
    }

    fn write(self: &Self, data: &[T]) {
        let write_head_ptr = self.mapped_ptr.expect("Buffer not mapped!");
        unsafe {
            write_head_ptr.copy_from_nonoverlapping(data.as_ptr(), data.len());
        }
    }
}

impl<T: Copy, const count: usize> Drop for GpuBuffer<T, count> {
    fn drop(&mut self) {
        let device = self.device.upgrade().unwrap();
        unsafe {
            device.device.destroy_buffer(self.buffer, None);
            device.device.free_memory(self.memory, None)
        }
    }
}

