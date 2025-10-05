
fn main() {
    println!("Hello, world!");
    
}

trait Backend {
    fn create_graphics_pipeline(self: &Self) -> &mut impl GraphicsPipeline;
    fn create_compute_pipeline(self: &Self) -> &mut impl ComputePipeline;
}
trait GraphicsPipeline {
    fn create_drawlet(self: &Self) -> &impl Drawlet;
}
trait ComputePipeline {}
trait Drawlet {}
