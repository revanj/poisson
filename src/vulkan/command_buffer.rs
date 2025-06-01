use std::ptr::null;
use ash::vk;
use crate::vulkan::Device;

pub struct OneshotCommandBuffer {
    pub command_buffer: vk::CommandBuffer,
}
impl OneshotCommandBuffer {
    pub fn new(device: &Device) -> Self {
        let command_buffers_alloc_info =
            vk::CommandBufferAllocateInfo::default()
                .command_buffer_count(1)
                .command_pool(device.command_pool)
                .level(vk::CommandBufferLevel::PRIMARY);
        let command_buffers = unsafe { device.device
            .allocate_command_buffers(&command_buffers_alloc_info)
            .unwrap()};
        let command_buffer = command_buffers[0];

        unsafe {
            device.device.reset_command_buffer(
                command_buffer,
                vk::CommandBufferResetFlags::RELEASE_RESOURCES,
            ).expect("Reset command buffer failed.");

            let command_buffer_begin_info
                = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

            device.device.begin_command_buffer(
                command_buffer,
                &command_buffer_begin_info
            ).expect("Begin command buffer failed");
        }

        Self {
            command_buffer
        }
    }

    pub fn submit(self: &Self, device: &Device) {
        unsafe {
            device.device.end_command_buffer(self.command_buffer)
                .expect("End command buffer failed");
        }

        let command_buffers = std::vec![self.command_buffer];

        let submit_info = vk::SubmitInfo::default()
            .command_buffers(&command_buffers);

        unsafe { device.device.queue_submit(
            device.present_queue,
            &[submit_info],
            vk::Fence::null()
        ).expect("queue submit failed.");
        device.device.device_wait_idle().unwrap(); }

    }
}
pub struct CommandBuffers {
    pub command_buffers: Vec<vk::CommandBuffer>
}

impl CommandBuffers {
    pub fn new(device: &Device, count: u32) -> Self {
        let command_buffers_alloc_info =
            vk::CommandBufferAllocateInfo::default()
            .command_buffer_count(count)
            .command_pool(device.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY);
        let command_buffers = unsafe { device.device
            .allocate_command_buffers(&command_buffers_alloc_info)
            .unwrap()};

        Self {
            command_buffers
        }
    }
}