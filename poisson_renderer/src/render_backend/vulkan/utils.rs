use ash::vk;

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

use cgmath::prelude::*;
use cgmath::{BaseFloat, Matrix4, Rad};

/// Perspective matrix that is suitable for Vulkan.
///
/// It inverts the projected y-axis. And set the depth range to 0..1
/// instead of -1..1. Mind the vertex winding order though.
pub fn perspective(fov: f32, aspect: f32, near: f32, far: f32, axis_alignment: [f32; 3]) -> Matrix4<f32>
{
    let two = 2f32;
    let f: f32 = 0.5f32/fov.tan();

    let c0r0 = f / aspect * axis_alignment[0];
    let c0r1 = 0f32;
    let c0r2 = 0f32;
    let c0r3 = 0f32;

    let c1r0 = 0f32;
    let c1r1 = f * axis_alignment[1];
    let c1r2 = 0f32;
    let c1r3 = 0f32;

    let c2r0 = 0f32;
    let c2r1 = 0f32;
    let c2r2 = far / (far - near) * axis_alignment[2];
    let c2r3 = 1f32 * axis_alignment[2];

    let c3r0 = 0f32;
    let c3r1 = 0f32;
    let c3r2 = -(far * near) / (far - near);
    let c3r3 = 0f32;

    #[cfg_attr(rustfmt, rustfmt_skip)]
    Matrix4::new(
        c0r0, c0r1, c0r2, c0r3,
        c1r0, c1r1, c1r2, c1r3,
        c2r0, c2r1, c2r2, c2r3,
        c3r0, c3r1, c3r2, c3r3,
    )
}