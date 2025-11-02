use std::sync::{Arc, Mutex};

fn main() {
    let p: Arc<Mutex<dyn Super>> = Arc::new(Mutex::new(Obj {}));
    let raw = Arc::into_raw(p) as *const Mutex<dyn Sub>;
    let q = unsafe { Arc::from_raw(raw) };
}

trait Super {}
trait Sub: Super {}

struct Obj {}
impl Super for Obj {}
impl Sub for Obj {}