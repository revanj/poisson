use cgmath::{Matrix4, Quaternion, SquareMatrix, Vector3, Vector4};

// this module will be a thin wrapper on cgmath
pub mod utils;

pub struct Transform {
    t: Vector3<f32>,
    r: Quaternion<f32>,
    s: Vector3<f32>
}

impl Transform {
    pub fn to_mat4(&self) -> cgmath::Matrix4<f32>{
        let rot_matrix: Matrix4<f32> = self.r.into();
        let scale_matrix =
            Matrix4::from_diagonal(
                Vector4::new(self.s[0], self.s[1], self.s[2], 1f32)
            );
        let mut ret = rot_matrix * scale_matrix;
        ret.w = Vector4::new(self.t[0], self.t[1], self.t[2], 1f32);
        
        ret
    }
}
