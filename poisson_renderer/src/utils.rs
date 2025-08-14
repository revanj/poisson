#[cfg(target_arch="wasm32")]
#[macro_export]
macro_rules! shader {
    ($x:expr) => {
            concat!($x, ".wgsl")
    }
}

#[cfg(not(target_arch="wasm32"))]
#[macro_export]
macro_rules! shader {
    ($x:literal) => {
            concat!($x, ".slang")
    }
}