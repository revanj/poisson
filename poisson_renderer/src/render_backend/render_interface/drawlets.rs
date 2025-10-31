use crate::render_backend::render_interface::RenderObject;
use crate::render_backend::RenderDrawlet;

pub trait DrawletTrait {}
pub trait TexturedMeshTrait: DrawletTrait {}
pub trait ColoredMeshTrait: DrawletTrait {}
