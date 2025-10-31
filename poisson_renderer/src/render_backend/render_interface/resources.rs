// thin wrappers on top of the sd

use std::any::Any;

pub trait GpuBufferTrait<T>: Any {
    fn get_size_bytes(&self) -> usize;
    fn get_count(&self) -> usize;
}


pub struct GpuBufferHandle<T> {
    pub(crate) buffer: rj::Own<dyn GpuBufferTrait<T>>
}
impl<T: 'static> GpuBufferHandle<T> {
    pub fn from_own(buffer: rj::Own<dyn GpuBufferTrait<T>>) -> Self {
        Self {
            buffer
        }
    }
    pub fn get_size_bytes(&self) -> usize {
        self.buffer.access().get_size_bytes()
    }
    pub fn get_count(&self) -> usize {
        self.buffer.access().get_count()
    }
}

pub trait GpuTextureTrait: Any {}
pub struct GpuTextureHandle {
    texture: rj::Own<dyn GpuTextureTrait>
}
impl GpuTextureHandle {

}