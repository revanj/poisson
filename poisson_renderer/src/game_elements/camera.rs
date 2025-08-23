use std::f32::consts::PI;
use cgmath::Point3;
use crate::math::Transform;

pub struct CameraIntrinsics {
    near: f32,
    far: f32,
    fov: f32,
    aspect: f32,
    axis_alignment: [f32; 3]
}

pub struct Camera {
    extr: Transform,
    intr: CameraIntrinsics
}

impl Camera {
    pub fn new_default(transform: Transform, axis_alignment: [f32; 3]) -> Self {
        let intr = CameraIntrinsics {
            near: 0.1f32,
            far: 10f32,
            fov: PI/4f32,
            aspect: 800f32/600f32,
            axis_alignment
        };
        
        Self::new(transform, intr)
    }
    
    pub fn new(extr: Transform, intr: CameraIntrinsics) -> Self {
        Self {
            extr,
            intr
        }
    }
    
}

