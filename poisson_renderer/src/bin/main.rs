use std::error::Error;

fn main() -> Result<(), impl Error> {
    poisson_renderer::run_wgpu()
}
