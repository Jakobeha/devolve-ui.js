use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;
use crate::core::data::rx::context::{RxContext, RxContextRef};

pub struct RunRxContext<'c, F: FnMut(&RxContextRef<'c>) + 'c>(RefCell<F>, PhantomData<&'c ()>);

impl <'c, F: FnMut(&RxContextRef<'c>) + 'c> RunRxContext<'c, F> {
    pub fn new(f: F) -> Self {
        RunRxContext(RefCell::new(f), PhantomData)
    }
}

impl<'c, F: FnMut(&RxContextRef<'c>) + 'c> RxContext for RunRxContext<'c, F> {
    fn recompute(self: Rc<Self>) {
        let self_ = self.clone();
        match self.0.try_borrow_mut() {
            Err(error) => {
                panic!("recompute triggered its own recompute: {}", error);
            }
            Ok(mut fun) => {
                fun(&RxContextRef::Strong(self_));
            }
        }
    }
}

/// Runs the function and re-runs every time one of its referenced dependencies changes.
pub fn run_rx<'c, F: FnMut(&RxContextRef<'c>) + 'c>(f: F) {
    let ctx_rc = Rc::new(RunRxContext::new(f));
    ctx_rc.recompute();
}
