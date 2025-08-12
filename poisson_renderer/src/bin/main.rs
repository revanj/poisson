use std::error::Error;
use poisson_renderer::{NothingGame};

fn main() -> Result<(), impl Error> {
    poisson_renderer::run_game::<NothingGame>()
}

