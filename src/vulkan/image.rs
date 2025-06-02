use ash::vk;
use ash::vk::CommandBuffer;
use crate::vulkan::{find_memorytype_index, record_submit_commandbuffer};
use crate::vulkan::command_buffer::OneshotCommandBuffer;
use crate::vulkan::device::Device;

pub struct Image {
    image: vk::Image,
    memory: vk::DeviceMemory,
    view: vk::ImageView,
}

impl Image {
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
    pub fn new_depth_image(device: &Device, extent: vk::Extent2D) -> Self {
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
        let depth_image_memory_index = find_memorytype_index(
            &depth_image_memory_req,
            &device.memory_properties,
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

        let depth_image_view = unsafe {device.device
            .create_image_view(&depth_image_view_info, None)
            .unwrap()};

        Self {
            image: depth_image,
            memory: depth_image_memory,
            view: depth_image_view,
        }

    }
}
//
    //
    //     record_submit_commandbuffer(
    //         &device,
    //         setup_command_buffer,
    //         setup_commands_reuse_fence,
    //         present_queue,
    //         &[],
    //         &[],
    //         &[],
    //         |device, setup_command_buffer| {
    //             let layout_transition_barriers = vk::ImageMemoryBarrier::default()
    //                 .image(depth_image)
    //                 .dst_access_mask(
    //                     vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
    //                         | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
    //                 )
    //                 .new_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
    //                 .old_layout(vk::ImageLayout::UNDEFINED)
    //                 .subresource_range(
    //                     vk::ImageSubresourceRange::default()
    //                         .aspect_mask(vk::ImageAspectFlags::DEPTH)
    //                         .layer_count(1)
    //                         .level_count(1),
    //                 );
    //
    //             device.cmd_pipeline_barrier(
    //                 setup_command_buffer,
    //                 vk::PipelineStageFlags::BOTTOM_OF_PIPE,
    //                 vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
    //                 vk::DependencyFlags::empty(),
    //                 &[],
    //                 &[],
    //                 &[layout_transition_barriers],
    //             );
    //         },
    //     );
    //
    //
    //
    //     let depth_image_view_info = vk::ImageViewCreateInfo::default()
    //         .subresource_range(
    //             vk::ImageSubresourceRange::default()
    //                 .aspect_mask(vk::ImageAspectFlags::DEPTH)
    //                 .level_count(1)
    //                 .layer_count(1),
    //         )
    //         .image(depth_image)
    //         .format(depth_image_create_info.format)
    //         .view_type(vk::ImageViewType::TYPE_2D);
    //
    //     let depth_image_view = device
    //         .create_image_view(&depth_image_view_info, None)
    //         .unwrap();
    //
    // }
