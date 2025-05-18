use std::rc::{Rc, Weak};
use std::cell::RefCell;

struct ContainingObject<'a> {
    pub obs: Vec<Rc<RefCell<Observer<'a>>>>,
    pub x: u32
}

impl ContainingObject<'_> {
    fn new() -> Self {
        let obs = Vec::new();
        Self {
            obs,
            x: 0
        }
    }
}

struct Observer<'a> {
   func: &'a mut dyn FnMut()
}

impl<'a> Observer<'a> {
    fn invoke(&mut self) {
        (self.func)();
    }

    fn new(func: &'a mut dyn FnMut()) -> Self {
        Observer { func }
    }

}

struct Observed<'a> {
    target: Weak<RefCell<Observer<'a>>>,
}
impl Observed<'_> {
    fn emit_signal(self: &Self) {
        self.target.upgrade().unwrap().borrow_mut().invoke();
        println!("emitted signal");
    }
}


fn main()  {
    let mut observer = ContainingObject::new();
    let mut binding = || {
            println!("got signal, modifying x");
            observer.x += 1;
    };
    observer.obs.push(Rc::new(RefCell::new(Observer {func: &mut binding})));

    let observed = Observed {
        target: Rc::downgrade(&observer.obs[0])
    };

    std::mem::drop(observed);

    observer.x += 1;
    println!("x is now, {}", observer.x);
}
