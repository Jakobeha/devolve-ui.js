use std::cell::RefCell;
use std::collections::HashSet;
use std::marker::PhantomData;
use crate::core::data::rx::context::{assert_is_ctx_variant, RxContextRef};

// /*pub(super) */pub struct RxObservers<'c>(UnsafeCovariantRefCell<HashSet<RxContextRef<'c>>>);
pub struct RxObservers<'c>(RefCell<HashSet<RxContextRef<'static>>>, PhantomData<&'c ()>);
assert_is_ctx_variant!((RxObservers<'c>) over 'c);

impl<'c> RxObservers<'c> {
    pub(super) fn new() -> Self {
        RxObservers(RefCell::new(HashSet::new()), PhantomData)
    }

    pub(super) fn insert(&self, c: RxContextRef<'c>) {
        self.0.borrow_mut().insert(unsafe { std::mem::transmute(c) });
    }

    pub(super) fn trigger(&self) {
        // We want to store in Vec first because a trigger will likely cause observers to change,
        // and we can't reborrow
        let mut result = Vec::new();

        // We drain because observers will re-add themselves on recompute
        for observer in self.0.borrow_mut().drain() {
            if let Some(observer) = observer.upgrade() {
                result.push(observer);
            }
        }

        for observer in result {
            observer.recompute();
        }
    }
}
