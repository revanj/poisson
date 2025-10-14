use cgmath::prelude::*;
use cgmath::{BaseFloat, Matrix4, Rad};

pub fn perspective(half_fov: f32, aspect: f32, near: f32, far: f32, axis_alignment: [f32; 3]) -> Matrix4<f32>
{
    let f: f32 = 0.5f32/ half_fov.tan();

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

pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32, axis_alignment: [f32; 3]) -> Matrix4<f32>
{
    let c0r0 = 2f32/(right - left) * axis_alignment[0];
    let c0r1 = 0f32;
    let c0r2 = 0f32;
    let c0r3 = 0f32;

    let c1r0 = 0f32;
    let c1r1 = 2f32/(bottom - top) * axis_alignment[1];
    let c1r2 = 0f32;
    let c1r3 = 0f32;

    let c2r0 = 0f32;
    let c2r1 = 0f32;
    let c2r2 = 2f32 / (far - near) * axis_alignment[2];
    let c2r3 = 0f32;

    let c3r0 = -(right + left)/(right - left);
    let c3r1 = -(bottom + top)/(bottom - top);
    let c3r2 = -(far + near) / (far - near);
    let c3r3 = 1f32;

    #[cfg_attr(rustfmt, rustfmt_skip)]
    Matrix4::new(
        c0r0, c0r1, c0r2, c0r3,
        c1r0, c1r1, c1r2, c1r3,
        c2r0, c2r1, c2r2, c2r3,
        c3r0, c3r1, c3r2, c3r3,
    )
}