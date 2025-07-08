use std::ffi::c_void;
use std::marker;
use std::marker::PhantomData;
use std::ptr::null;
use std::sync::Arc;
use ash::util::Align;
use ash::vk;
use ash::vk::{BufferUsageFlags, DeviceSize, SharingMode};
use crate::vulkan::device::Device;
use crate::vulkan::utils;

pub enum BufferType {
    Uniform,
}


// a buffer on the GPU that is a list of a fixed type
pub struct GpuBuffer<T: Copy> {
    pub device: std::sync::Weak<Device>,
    pub buffer: vk::Buffer,
    pub allocated_size: vk::DeviceSize,
    pub memory: vk::DeviceMemory,
    pub mapped_slice: Option<Align<T>>,
}


impl<T: Copy> GpuBuffer<T> {
    const ALIGNED_ELEMENT_SIZE: DeviceSize = align_of::<T>() as DeviceSize;
    pub fn create_mapped_uniform(device: &Arc<Device>, count: usize) -> GpuBuffer<T> {
        let create_info = vk::BufferCreateInfo {
            size: Self::ALIGNED_ELEMENT_SIZE * (count as DeviceSize),
            usage: BufferUsageFlags::UNIFORM_BUFFER,
            sharing_mode: SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let buffer = unsafe { device.device.create_buffer(&create_info, None) }.unwrap();
        let mem_requirements = unsafe { device.device.get_buffer_memory_requirements(buffer) };

        let physical_memory_properties = device.physical_memory_properties;

        let memory_type = utils::find_memorytype_index(
            &mem_requirements, &physical_memory_properties,
            vk::MemoryPropertyFlags::HOST_VISIBLE
                | vk::MemoryPropertyFlags::HOST_COHERENT).unwrap();

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

        // let mapped_slice = unsafe { Align::new(
        //     mapped_ptr, align_of::<T>() as DeviceSize,
        //     mem_requirements.size
        // ) };

        let mut ret = Self {
            device: Arc::downgrade(device),
            buffer,
            allocated_size: mem_requirements.size,
            memory,
            mapped_slice: None,
        };
        
        ret.map();
        
        ret
    }

    pub fn map(self: &mut Self) {
        let device = self.device.upgrade().unwrap();
        let ptr = unsafe {
            device.device.map_memory(
                self.memory, 0, self.allocated_size, vk::MemoryMapFlags::empty())
        }.unwrap();
        
        let slice: Align<T> = unsafe { Align::new(
            ptr, align_of::<T>() as DeviceSize,
            self.allocated_size
        ) };
        
        self.mapped_slice = Some(slice);
    }
    
    pub fn unmap(self: &mut Self) {
        let device = self.device.upgrade().unwrap();
        unsafe {
            device.device.unmap_memory(self.memory);
            self.mapped_slice = None;
        }
    }

    pub fn write(self: &mut Self, data: &[T]) {
        if let Some(slice) = self.mapped_slice.as_mut() {
            slice.copy_from_slice(data);
        } else {
            panic!("writing to unmapped buffer!");
        }
    }

    pub fn create_buffer(
        device: &Arc<Device>,
        usage: vk::BufferUsageFlags,
        memory_property: vk::MemoryPropertyFlags,
        count: usize) -> GpuBuffer<T>
    {
        let buffer_create_info = vk::BufferCreateInfo::default()
            .size((size_of::<T>() * count) as DeviceSize)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe {
            device.device.create_buffer(&buffer_create_info, None)
        }.unwrap();

        let memory_req = unsafe {
            device.device.get_buffer_memory_requirements(buffer)
        };

        let memory_index = utils::find_memorytype_index(
            &memory_req,
            &device.physical_memory_properties,
            memory_property
        ).expect("Unable to find suitable memory type");

        let allocate_info = vk::MemoryAllocateInfo {
            allocation_size: memory_req.size,
            memory_type_index: memory_index,
            ..Default::default()
        };

        let buffer_memory = unsafe {
            device.device.allocate_memory(&allocate_info, None)
        }.unwrap();
        
        unsafe {
            device.device.bind_buffer_memory(buffer, buffer_memory, 0).unwrap();
        }
        
        Self {
            device: Arc::downgrade(device),
            buffer,
            allocated_size: memory_req.size,
            memory: buffer_memory,
            mapped_slice: None
        }
    }
}

impl<T: Copy> Drop for GpuBuffer<T> {
    fn drop(&mut self) {
        let device = self.device.upgrade().unwrap();
        unsafe {
            device.device.free_memory(self.memory, None);
            device.device.destroy_buffer(self.buffer, None);
        }
    }
}

