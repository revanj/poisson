
use ash::{
    ext::debug_utils,
    khr::{surface, wayland_surface, swapchain},
    vk, Device, Entry, Instance,
};
/// Get required instance extensions.
/// This is windows specific.
pub fn required_extension_names() -> Vec<*const i8> {
    vec![surface::NAME.as_ptr(), wayland_surface::NAME.as_ptr()]
}
