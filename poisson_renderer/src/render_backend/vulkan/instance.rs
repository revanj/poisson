use std::borrow::Cow;
use std::ffi;
use std::ffi::c_char;
use std::sync::Arc;
use ash;
use ash::vk;
use ash_window;
use ash::ext::debug_utils as ash_debug_utils;
use ash::vk::DebugUtilsMessengerEXT;
use winit::window::Window;
use winit::raw_window_handle::HasDisplayHandle;

use crate::render_backend::vulkan::Destroy;

pub struct Instance {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    debug_utils_loader: ash_debug_utils::Instance,
    debug_callback: DebugUtilsMessengerEXT,
}

impl Instance {
    pub fn new(window: &Arc<dyn Window>) -> Self {
        println!("Creating Vulkan context");
        let entry = unsafe { ash::Entry::load().unwrap() };
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

        let instance = unsafe { entry.create_instance(&create_info, None).unwrap() };
        let debug_utils_loader = ash_debug_utils::Instance::new(&entry, &instance);

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
            .pfn_user_callback(Some(Self::vulkan_debug_callback));

        let debug_callback = unsafe {
            debug_utils_loader.create_debug_utils_messenger(&debug_info, None)
            .unwrap()
        };

        Self {
            entry,
            instance,
            debug_utils_loader,
            debug_callback
        }
    }

    fn create_device(self: &mut Self) {

    }

    unsafe extern "system" fn vulkan_debug_callback(
        message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
        message_type: vk::DebugUtilsMessageTypeFlagsEXT,
        p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
        _user_data: *mut std::os::raw::c_void,
    ) -> vk::Bool32 {
        unsafe {
            let callback_data = *p_callback_data;
            let message_id_number = callback_data.message_id_number;

            let message_id_name =
                if callback_data.p_message_id_name.is_null()
                { Cow::from("") } else
                { ffi::CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy() };

            let message =
                if callback_data.p_message.is_null()
                { Cow::from("") } else
                { ffi::CStr::from_ptr(callback_data.p_message).to_string_lossy() };

            println!(
                "{message_severity:?}:{message_type:?} \
                [{message_id_name} ({message_id_number})] : {message}");
        }
        vk::FALSE
    }
}

impl Destroy for Instance {
    fn destroy(&mut self) {
        unsafe {
            self.debug_utils_loader.destroy_debug_utils_messenger(self.debug_callback, None);
            self.instance.destroy_instance(None);
        }
    }
}