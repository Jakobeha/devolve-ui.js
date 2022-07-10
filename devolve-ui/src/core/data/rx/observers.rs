use std::cell::RefCell;
use std::collections::HashSet;
use crate::core::data::rx::context::RxContextRef;

pub(super) struct RxObservers<'c>(RefCell<HashSet<RxContextRef<'c>>>);

impl<'c> RxObservers<'c> {
    pub(super) fn new() -> Self {
        RxObservers(RefCell::new(HashSet::new()))
    }

    pub(super) fn insert(&self, c: RxContextRef<'c>) {
        self.0.borrow_mut().insert(c);
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
