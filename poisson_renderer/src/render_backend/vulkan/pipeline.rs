use std::ffi::CStr;
use std::sync::Arc;
use crate::render_backend::vulkan::device::Device;

pub struct GraphicsPipeline {

}

impl GraphicsPipeline {
    pub fn new(device: &Arc<Device>, bytecode: &[u32], vertex_name: &CStr, fragment_name: &CStr) {
        
    }
}