use std::sync::Arc;
use ash::vk;
use ash::vk::{CommandBuffer, ImageAspectFlags, ImageTiling};


use crate::render_backend::vulkan;
use vulkan::{utils};
use vulkan::command_buffer::OneshotCommandBuffer;
use vulkan::device::Device;

pub struct Image {
    pub device: std::sync::Weak<Device>,
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
    pub view: vk::ImageView,
}

impl Image {
    fn copy_buffer_to_image(device: &Arc<Device>, buffer: &vk::Buffer, image: &vk::Image, extent: vk::Extent2D) {
        let cmd_buffer = OneshotCommandBuffer::new(device);
        let region = vk::BufferImageCopy::default()
            .buffer_offset(0)
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(
                vk::ImageSubresourceLayers::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .mip_level(0)
                    .base_array_layer(0)
                    .layer_count(1)
            )
            .image_offset(vk::Offset3D {x: 0, y: 0, z: 0})
            .image_extent(extent.into());

        unsafe {
            device.device.cmd_copy_buffer_to_image(
                cmd_buffer.command_buffer,
                *buffer,
                *image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[region]
            )
        }
        cmd_buffer.submit(device);
    }

    fn transition_image_layout(
        device: &Device,
        image: vk::Image,
        format: vk::Format,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout)
    {
        let cmd_buffer = OneshotCommandBuffer::new(device);
        let mut layout_transition_barrier =
            vk::ImageMemoryBarrier::default()
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(image).dst_access_mask(
                    vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                    | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
                )
                .new_layout(new_layout)
                .old_layout(old_layout)
                .subresource_range(
                    vk::ImageSubresourceRange::default()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .base_mip_level(0)
                        .level_count(1)
                        .base_array_layer(0)
                        .layer_count(1)
                );
        let mut src_stage = vk::PipelineStageFlags::default();
        let mut dst_stage = vk::PipelineStageFlags::default();
        match (old_layout, new_layout) {
            (vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL) => {
                layout_transition_barrier.src_access_mask = vk::AccessFlags::default();
                layout_transition_barrier.dst_access_mask = vk::AccessFlags::TRANSFER_WRITE;
                src_stage = vk::PipelineStageFlags::TOP_OF_PIPE;
                dst_stage = vk::PipelineStageFlags::TRANSFER;
            },
            (vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL) => {
                layout_transition_barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
                layout_transition_barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;
                src_stage = vk::PipelineStageFlags::TRANSFER;
                dst_stage = vk::PipelineStageFlags::FRAGMENT_SHADER;
            }
            _ => panic!("Unsupported image layout in transfer!")
        }

        unsafe {
            device.device.cmd_pipeline_barrier(
                cmd_buffer.command_buffer,
                src_stage,
                dst_stage,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[layout_transition_barrier],
            );
        }
        cmd_buffer.submit(device);
    }

    unsafe fn transition_depth_layout(
        device: &Device,
        command_buffer: CommandBuffer,
        image: vk::Image)
    {
        let layout_transition_barriers =
            vk::ImageMemoryBarrier::default().image(image).dst_access_mask(
                vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                    | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            )
            .new_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::DEPTH)
                    .layer_count(1)
                    .level_count(1),
            );
        unsafe {
            device.device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[layout_transition_barriers],
            );
        }
    }
    
    pub fn create_image_view(
        device: &Arc<Device>,
        image: &vk::Image,
        format: vk::Format,
        aspect: vk::ImageAspectFlags
    ) -> vk::ImageView {
        let image_view_info = vk::ImageViewCreateInfo::default()
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(aspect)
                    .level_count(1)
                    .layer_count(1),
            )
            .image(*image)
            .format(format)
            .view_type(vk::ImageViewType::TYPE_2D);

        let image_view = unsafe {device.device
            .create_image_view(&image_view_info, None)
            .unwrap()};
        
        image_view
    }

    pub fn create_image(
        device: &Arc<Device>,
        buffer: &vk::Buffer,
        format: vk::Format,
        tiling: vk::ImageTiling,
        usage: vk::ImageUsageFlags,
        memory_property: vk::MemoryPropertyFlags,
        extent: vk::Extent2D,
    ) -> Self {
        let image_create_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(extent.into())
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(tiling)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let image = unsafe {device.device
            .create_image(&image_create_info, None).unwrap()};
        let image_memory_req = unsafe {device.device
            .get_image_memory_requirements(image)};
        let image_memory_index = utils::find_memorytype_index(
            &image_memory_req,
            &device.physical_memory_properties,
            memory_property,
        ).expect("Unable to find suitable memory index for image.");

        let image_allocate_info = vk::MemoryAllocateInfo::default()
            .allocation_size(image_memory_req.size)
            .memory_type_index(image_memory_index);

        let memory = unsafe { device.device
            .allocate_memory(&image_allocate_info, None)
            .unwrap() };

        unsafe { device.device
            .bind_image_memory(image, memory, 0)
            .expect("Unable to bind image memory"); }
        
        Self::transition_image_layout(
            device,
            image,
            vk::Format::R8G8B8A8_SRGB,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL
        );
        Self::copy_buffer_to_image(
            device,
            buffer,
            &image,
            extent);
        Self::transition_image_layout(
            device,
            image, vk::Format::R8G8B8A8_SRGB,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
        
        let view = 
            Self::create_image_view(
                device, 
                &image, 
                image_create_info.format, 
                ImageAspectFlags::COLOR);

        Self {
            device: Arc::downgrade(&device),
            image,
            memory,
            view,
        }
    }

    pub fn new_depth_image(device: &Arc<Device>, extent: vk::Extent2D) -> Self {
        let depth_image_create_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::D16_UNORM)
            .extent(extent.into())
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let depth_image = unsafe {device.device
            .create_image(&depth_image_create_info, None).unwrap()};
        let depth_image_memory_req = unsafe {device.device
            .get_image_memory_requirements(depth_image)};
        let depth_image_memory_index = utils::find_memorytype_index(
            &depth_image_memory_req,
            &device.physical_memory_properties,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        ).expect("Unable to find suitable memory index for depth image.");

        let depth_image_allocate_info = vk::MemoryAllocateInfo::default()
            .allocation_size(depth_image_memory_req.size)
            .memory_type_index(depth_image_memory_index);

        let depth_image_memory = unsafe { device.device
            .allocate_memory(&depth_image_allocate_info, None)
            .unwrap() };

        unsafe { device.device
            .bind_image_memory(depth_image, depth_image_memory, 0)
            .expect("Unable to bind depth image memory"); }

        let setup_cmd_buffer = OneshotCommandBuffer::new(&device);
        unsafe {
            Self::transition_depth_layout(
                &device,
                setup_cmd_buffer.command_buffer,
                depth_image);
        }
        setup_cmd_buffer.submit(&device);

        let depth_image_view_info = vk::ImageViewCreateInfo::default()
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::DEPTH)
                    .level_count(1)
                    .layer_count(1),
            )
            .image(depth_image)
            .format(depth_image_create_info.format)
            .view_type(vk::ImageViewType::TYPE_2D);
        
        let depth_image_view = Self::create_image_view(
            device, 
            &depth_image, 
            depth_image_create_info.format, 
            ImageAspectFlags::DEPTH
        );

        Self {
            device: Arc::downgrade(&device),
            image: depth_image,
            memory: depth_image_memory,
            view: depth_image_view,
        }
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        let device = self.device.upgrade().unwrap();
        unsafe {
            device.device.free_memory(self.memory, None);
            device.device.destroy_image_view(self.view, None);
            device.device.destroy_image(self.image, None);
        }
    }
}
