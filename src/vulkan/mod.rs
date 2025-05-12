
mod utils;
use std::ops::Drop;
use ash::vk;
use std::ffi;
use std::borrow::Cow;
use ash::ext::debug_utils as ash_debug_utils;
use ash::khr::{surface as ash_surface, surface, swapchain as ash_swapchain};
use ash::vk::DebugUtilsMessengerEXT;
use winit::raw_window_handle::{HasDisplayHandle, HasRawDisplayHandle, HasWindowHandle, RawDisplayHandle};
use std::ffi::c_char;
use ash_window;
use winit::window::Window;

const ENABLE_VALIDATION_LAYERS: bool = true;
const REQUIRED_LAYERS: [&'static str; 1] = ["VK_LAYER_LUNARG_standard_validation"];

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 { 
    unsafe {
        let callback_data = *p_callback_data;
        let message_id_number = callback_data.message_id_number;

        let message_id_name = if callback_data.p_message_id_name.is_null() {
            Cow::from("")
        } else {
            ffi::CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
        };

        let message = if callback_data.p_message.is_null() {
            Cow::from("")
        } else {
            ffi::CStr::from_ptr(callback_data.p_message).to_string_lossy()
        };

        println!(
            "{message_severity:?}:\n{message_type:?} [{message_id_name} ({message_id_number})] : {message}",
        );
    }
    vk::FALSE
}


pub fn find_memorytype_index(
    memory_req: &vk::MemoryRequirements,
    memory_prop: &vk::PhysicalDeviceMemoryProperties,
    flags: vk::MemoryPropertyFlags,
) -> Option<u32> {
    memory_prop.memory_types[..memory_prop.memory_type_count as _]
        .iter()
        .enumerate()
        .find(|(index, memory_type)| {
            (1u32 << index) & memory_req.memory_type_bits != 0
                && memory_type.property_flags & flags == flags
        })
        .map(|(index, _memory_type)| index as _)
}

/// Helper function for submitting command buffers. Immediately waits for the fence before the command buffer
/// is executed. That way we can delay the waiting for the fences by 1 frame which is good for performance.
/// Make sure to create the fence in a signaled state on the first use.
#[allow(clippy::too_many_arguments)]
pub fn record_submit_commandbuffer<F: FnOnce(&ash::Device, vk::CommandBuffer)>(
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    command_buffer_reuse_fence: vk::Fence,
    submit_queue: vk::Queue,
    wait_mask: &[vk::PipelineStageFlags],
    wait_semaphores: &[vk::Semaphore],
    signal_semaphores: &[vk::Semaphore],
    f: F,
) {
    unsafe {
        device
            .wait_for_fences(&[command_buffer_reuse_fence], true, u64::MAX)
            .expect("Wait for fence failed.");

        device
            .reset_fences(&[command_buffer_reuse_fence])
            .expect("Reset fences failed.");

        device
            .reset_command_buffer(
                command_buffer,
                vk::CommandBufferResetFlags::RELEASE_RESOURCES,
            )
            .expect("Reset command buffer failed.");

        let command_buffer_begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        device
            .begin_command_buffer(command_buffer, &command_buffer_begin_info)
            .expect("Begin commandbuffer");
        f(device, command_buffer);
        device
            .end_command_buffer(command_buffer)
            .expect("End commandbuffer");

        let command_buffers = vec![command_buffer];

        let submit_info = vk::SubmitInfo::default()
            .wait_semaphores(wait_semaphores)
            .wait_dst_stage_mask(wait_mask)
            .command_buffers(&command_buffers)
            .signal_semaphores(signal_semaphores);

        device
            .queue_submit(submit_queue, &[submit_info], command_buffer_reuse_fence)
            .expect("queue submit failed.");
    }
}
/// Vulkan Context which contains physical device, logical device, and surface, etc.
/// There will probably be a pointer of this being passed around

pub struct VulkanContext {
    entry : ash::Entry,
    instance : ash::Instance,
    debug_utils_loader: ash_debug_utils::Instance,
    debug_callback: DebugUtilsMessengerEXT,
    device : ash::Device,
    surface_loader: ash::khr::surface::Instance,
    swapchain_loader: ash::khr::swapchain::Device,
    physical_device: vk::PhysicalDevice,
    device_memory_properties: vk::PhysicalDeviceMemoryProperties,
    queue_family_index: u32,
    present_queue: vk::Queue,

    surface: vk::SurfaceKHR,
    surface_format: vk::SurfaceFormatKHR,
    surface_resolution: vk::Extent2D,

    swapchain: vk::SwapchainKHR,
    swapchain_dirty: bool,

    present_images: Vec<vk::Image>,
    present_image_views: Vec<vk::ImageView>,

    pool: vk::CommandPool,
    draw_command_buffer: vk::CommandBuffer,
    setup_command_buffer: vk::CommandBuffer,

    depth_image: vk::Image,
    depth_image_view: vk::ImageView,
    depth_image_memory: vk::DeviceMemory,

    present_complete_semaphore: vk::Semaphore,
    rendering_complete_semaphore: vk::Semaphore,

    draw_commands_reuse_fence: vk::Fence,
    setup_commands_reuse_fence: vk::Fence,
}

impl VulkanContext {
    pub unsafe fn new(window: &Box<dyn Window>) -> Self {
        println!("Creating Vulkan context");
        use ash::{vk, Entry};
        let entry = unsafe { Entry::load().unwrap() };
        let app_info = vk::ApplicationInfo {
            api_version: vk::make_api_version(0, 1, 0, 0),
            ..Default::default()
        };

        let layer_names = [c"VK_LAYER_KHRONOS_validation"];
        let layers_names_raw: Vec<*const c_char> = layer_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();

        let mut extension_names =
            ash_window::enumerate_required_extensions(window.display_handle().unwrap().as_raw()).unwrap().to_vec();
        extension_names.push(ash_debug_utils::NAME.as_ptr());

        let create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_layer_names(&layers_names_raw)
            .enabled_extension_names(&extension_names)
            .flags(vk::InstanceCreateFlags::default());


        let instance = entry.create_instance(&create_info, None).unwrap();

        let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(Some(vulkan_debug_callback));

        let debug_utils_loader = ash_debug_utils::Instance::new(&entry, &instance);

        let debug_callback = debug_utils_loader
                .create_debug_utils_messenger(&debug_info, None)
                .unwrap();

        let surface =
            ash_window::create_surface(
                &entry, &instance,
                window.display_handle().unwrap().as_raw(),
                window.window_handle().unwrap().as_raw(),
            None).unwrap();

        let physical_devices = instance.enumerate_physical_devices().expect("Failed to enumerate physical devices");
        let surface_loader = surface::Instance::new(&entry, &instance);
        let mut selected_device = None;

        for physical_device in physical_devices.iter() {
            let queue_family_properties =
                instance.get_physical_device_queue_family_properties(*physical_device);

            for (index, info) in queue_family_properties.iter().enumerate() {
                let supports_graphics = info.queue_flags.contains(vk::QueueFlags::GRAPHICS);
                let supports_surface = surface_loader
                    .get_physical_device_surface_support(*physical_device, index as u32, surface)
                    .unwrap();

                if supports_graphics && supports_surface {
                    selected_device = Some((*physical_device, index));
                    break;
                }
            }

            if selected_device.is_some() {
                break;
            }
        }

        let (physical_device, queue_family_index) = selected_device
            .expect("Failed to find suitable physical device");

        let queue_family_index = queue_family_index as u32;

        let device_extension_names_raw = [
            ash_swapchain::NAME.as_ptr(),
        ];

        let features = vk::PhysicalDeviceFeatures {
            shader_clip_distance: 1,
            ..Default::default()
        };
        let priorities = [1.0];

        let queue_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family_index)
            .queue_priorities(&priorities);

        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(std::slice::from_ref(&queue_info))
            .enabled_extension_names(&device_extension_names_raw)
            .enabled_features(&features);

        let device: ash::Device = instance
            .create_device(physical_device, &device_create_info, None)
            .unwrap();

        let present_queue = device.get_device_queue(queue_family_index, 0);

        let surface_format = surface_loader
            .get_physical_device_surface_formats(physical_device, surface)
            .unwrap()[0];


        let present_queue = device.get_device_queue(queue_family_index, 0);

        let surface_format = surface_loader
            .get_physical_device_surface_formats(physical_device, surface)
            .unwrap()[0];

        let surface_capabilities = surface_loader
            .get_physical_device_surface_capabilities(physical_device, surface)
            .unwrap();

        let mut desired_image_count = surface_capabilities.min_image_count + 1;
        if surface_capabilities.max_image_count > 0
            && desired_image_count > surface_capabilities.max_image_count
        {
            desired_image_count = surface_capabilities.max_image_count;
        }
        let window_size = window.surface_size();
        let surface_resolution = match surface_capabilities.current_extent.width {
            u32::MAX => vk::Extent2D {
                width: window_size.width,
                height: window_size.height,
            },
            _ => surface_capabilities.current_extent,
        };

        let pre_transform = if surface_capabilities
            .supported_transforms
            .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
        {
            vk::SurfaceTransformFlagsKHR::IDENTITY
        } else {
            surface_capabilities.current_transform
        };
        let present_modes = surface_loader
            .get_physical_device_surface_present_modes(physical_device, surface)
            .unwrap();
        let present_mode = present_modes
            .iter()
            .cloned()
            .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
            .unwrap_or(vk::PresentModeKHR::FIFO);
        let swapchain_loader = ash_swapchain::Device::new(&instance, &device);

        let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(desired_image_count)
            .image_color_space(surface_format.color_space)
            .image_format(surface_format.format)
            .image_extent(surface_resolution)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(pre_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .image_array_layers(1);

        let swapchain = swapchain_loader
            .create_swapchain(&swapchain_create_info, None)
            .unwrap();

        let pool_create_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(queue_family_index);

        let pool = device.create_command_pool(&pool_create_info, None).unwrap();

        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_buffer_count(2)
            .command_pool(pool)
            .level(vk::CommandBufferLevel::PRIMARY);

        let command_buffers = device
            .allocate_command_buffers(&command_buffer_allocate_info)
            .unwrap();
        let setup_command_buffer = command_buffers[0];
        let draw_command_buffer = command_buffers[1];

        let present_images = swapchain_loader.get_swapchain_images(swapchain).unwrap();
        let present_image_views: Vec<vk::ImageView> = present_images
            .iter()
            .map(|&image| {
                let create_view_info = vk::ImageViewCreateInfo::default()
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(surface_format.format)
                    .components(vk::ComponentMapping {
                        r: vk::ComponentSwizzle::R,
                        g: vk::ComponentSwizzle::G,
                        b: vk::ComponentSwizzle::B,
                        a: vk::ComponentSwizzle::A,
                    })
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .image(image);
                device.create_image_view(&create_view_info, None).unwrap()
            })
            .collect();

        let device_memory_properties = instance.get_physical_device_memory_properties(physical_device);
        let depth_image_create_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::D16_UNORM)
            .extent(surface_resolution.into())
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let depth_image = device.create_image(&depth_image_create_info, None).unwrap();
        let depth_image_memory_req = device.get_image_memory_requirements(depth_image);
        let depth_image_memory_index = find_memorytype_index(
            &depth_image_memory_req,
            &device_memory_properties,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )
            .expect("Unable to find suitable memory index for depth image.");

        let depth_image_allocate_info = vk::MemoryAllocateInfo::default()
            .allocation_size(depth_image_memory_req.size)
            .memory_type_index(depth_image_memory_index);

        let depth_image_memory = device
            .allocate_memory(&depth_image_allocate_info, None)
            .unwrap();

        device
            .bind_image_memory(depth_image, depth_image_memory, 0)
            .expect("Unable to bind depth image memory");

        let fence_create_info =
            vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);

        let draw_commands_reuse_fence = device
            .create_fence(&fence_create_info, None)
            .expect("Create fence failed.");
        let setup_commands_reuse_fence = device
            .create_fence(&fence_create_info, None)
            .expect("Create fence failed.");

        record_submit_commandbuffer(
            &device,
            setup_command_buffer,
            setup_commands_reuse_fence,
            present_queue,
            &[],
            &[],
            &[],
            |device, setup_command_buffer| {
                let layout_transition_barriers = vk::ImageMemoryBarrier::default()
                    .image(depth_image)
                    .dst_access_mask(
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

                device.cmd_pipeline_barrier(
                    setup_command_buffer,
                    vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                    vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[layout_transition_barriers],
                );
            },
        );

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

        let depth_image_view = device
            .create_image_view(&depth_image_view_info, None)
            .unwrap();

        let semaphore_create_info = vk::SemaphoreCreateInfo::default();

        let present_complete_semaphore = device
            .create_semaphore(&semaphore_create_info, None)
            .unwrap();
        let rendering_complete_semaphore = device
            .create_semaphore(&semaphore_create_info, None)
            .unwrap();


        Self {
            entry,
            instance,
            debug_utils_loader,
            debug_callback,
            device,
            surface_loader,
            swapchain_loader,
            physical_device,
            device_memory_properties,
            queue_family_index,
            present_queue,
            surface,
            surface_format,
            surface_resolution,

            swapchain,
            swapchain_dirty: false,

            present_images,
            present_image_views,

            pool,
            draw_command_buffer,
            setup_command_buffer,
            depth_image,
            depth_image_view,
            depth_image_memory,

            present_complete_semaphore,
            rendering_complete_semaphore,

            draw_commands_reuse_fence,
            setup_commands_reuse_fence,
        }
    }
    pub fn notify_window_resized(self: &mut Self) {
        self.swapchain_dirty = true;
    }
}

impl Drop for VulkanContext {
    fn drop(&mut self) {
        println!("Dropping application.");
        unsafe {
            self.device.device_wait_idle().unwrap();
            self.device.destroy_semaphore(self.present_complete_semaphore, None);
            self.device.destroy_semaphore(self.rendering_complete_semaphore, None);
            self.device.destroy_fence(self.draw_commands_reuse_fence, None);
            self.device.destroy_fence(self.setup_commands_reuse_fence, None);

            self.device.free_memory(self.depth_image_memory, None);
            self.device.destroy_image_view(self.depth_image_view, None);
            self.device.destroy_image(self.depth_image, None);

            for &image_view in self.present_image_views.iter() {
                self.device.destroy_image_view(image_view, None);
            }

            self.device.destroy_command_pool(self.pool, None);

            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.debug_utils_loader
                .destroy_debug_utils_messenger(self.debug_callback, None);
            self.instance.destroy_instance(None);
        }
    }
}
