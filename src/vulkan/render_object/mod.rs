use poisson_macros::ShaderInput;
use crate::slang;

pub trait ShaderInput{
    fn reflect() -> Vec<(&'static str, &'static str)>;
}

pub trait Draw {
    fn draw();
}

pub trait GraphicsShaderValidate {
    type Varying: ShaderInput;
    type Uniform: ShaderInput;
    
}


#[derive(Clone, Debug, Copy, ShaderInput)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub color: [f32; 3],
}

pub struct Graphics<Varying: ShaderInput, Uniform: ShaderInput> {
    pub varying: Varying,
    pub uniform: Uniform,
}

// impl<Varying, Uniform> Graphics<Varying, Uniform>
//     where Varying: ShaderInput, Uniform: ShaderInput
// {
//     fn new(shader_program: slang::LinkedProgram) -> Self {
//
//         Self {
//
//         }
//     }
// }

#[test]
fn test() {
    println!("{:?}", Vertex::reflect());
}