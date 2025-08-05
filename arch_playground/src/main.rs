#![feature(adt_const_params)]

use std::marker::ConstParamTy;

fn main() {
    println!("Hello, world!");
    
}

#[derive(Eq, PartialEq, ConstParamTy)]
enum BackendType
{
    Vulkan,
    Wgpu
}

const VK :BackendType = BackendType::Vulkan;

use BackendType::Vulkan;

trait RenderBackend<const TYPE: BackendType> {}

struct VulkanBackend {
    
}

impl RenderBackend<VK> for VulkanBackend {}