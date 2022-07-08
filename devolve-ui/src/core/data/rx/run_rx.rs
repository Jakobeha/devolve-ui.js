use crate::core::data::rx::context::{RxContext, RxContextRef};

pub struct RunRxContext<F: Fn(&RxContextRef)>(F);

impl<F: Fn(&RxContextRef)> RxContext for RunRxContext<F> {
    fn recompute(&self) {
        (self.0)(&RxContextRef::owned(self));
    }
}

/// Runs the function and re-runs every time one of its referenced dependencies changes.
pub fn run_rx(f: impl Fn(&RxContextRef)) {
    f(&RxContextRef::owned(RunRxContext(f)))
}
