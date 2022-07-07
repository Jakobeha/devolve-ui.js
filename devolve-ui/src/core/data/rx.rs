//! `Rx` means "reactive value" (or "reactive X"). It is a wrapper for a value which changes,
//! and these changes trigger dependencies to re-run and change themselves. You can't access the
//! value directly, instead you use an associated function like `with_rx` to access it in a closure
//! which can re-run whenever the dependency changes.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

pub trait RxContext {
    fn _recompute(self: Rc<Self>);
}

// Dumb dynamic object restriction
impl dyn RxContext {
    pub fn recompute(self: &Rc<Self>) {
        self.clone()._recompute()
    }
}

impl <T> dyn RxContext2<T=T> {
    pub fn recompute(self: &Rc<Self>) {
        self.clone()._recompute()
    }
}

pub trait RxContext2: RxContext {
    type T;

    fn replace(self: Rc<Self>, new_value: Self::T) -> Self::T;
}

pub struct RxContextImpl<T, F: Fn(&Weak<dyn RxContext>) -> T> {
    value: RefCell<T>,
    compute: F
}

union RxBody<T, const IS_MUTABLE: bool> {
    immutable: Rc<dyn RxContext2<T=T>>,
    mutable: T
}

pub struct Rx<T, const IS_MUTABLE: bool> {
    value: RxBody<T, IS_MUTABLE>,
    observers: RefCell<Vec<Weak<dyn RxContext>>>
}

pub trait AsRxContext {
    fn as_rx_context(&self) -> Weak<dyn RxContext>;
}

pub type IRx<T> = Rx<T, false>;
pub type MRx<T> = Rx<T, true>;

impl <T, const IS_MUTABLE: bool> Rx<T, IS_MUTABLE> {
    pub fn get(&self, c: &dyn AsRxContext) -> &T {
        self.observers.borrow_mut().push(c.as_rx_context());
        &self.value
    }

    pub fn map<U>(&self, f: impl Fn(&T) -> &U) -> IRx<U> {
        with_rx(|c| f(self.get(c)))
    }

    pub fn split_map2<U1, U2>(&self, f: impl Fn(&T) -> (&U1, &U2)) -> (IRx<U1>, IRx<U2>) {
        let (a, b) = f(self.get(&self));
    }

    pub fn split_map3<U1, U2, U3>(&self, f: impl Fn(&T) -> (&U1, &U2, &U3)) -> (IRx<U1>, IRx<U2>, IRx<U3>) {}

    pub fn split_map4<U1, U2, U3, U4>(&self, f: impl Fn(&T) -> (&U1, &U2, &U3, &U4)) -> (IRx<U1>, IRx<U2>, IRx<U3>, IRx<U4>) {}

    fn recompute(&self) {
        if !IS_MUTABLE {
            unsafe { self.value.immutable.recompute() }
        }
        for observer in self.observers.borrow().iter() {
            if let Some(observer) = observer.upgrade() {
                observer.recompute();
            }
        }
    }
}

impl <T> Rx<T, false> {
    fn new(compute: impl Fn(&Weak<dyn RxContext>) -> T) -> Self {
        Rx {
            value: RxBody { immutable: RxContextImpl::new(compute) },
            observers: RefCell::new(Vec::new())
        }
    }
}

impl <T> Rx<T, true> {
    fn new(initial: T) -> Self {
        Rx {
            value: initial,
            observers: RefCell::new(Vec::new())
        }
    }

    fn set(&mut self, new_value: T) {
        unsafe { self.value.mutable = new_value };
        self.recompute();
    }

    pub fn map_mut<U>(&mut self, f: impl Fn(&mut T) -> &mut U) -> MRx<U> {

    }

    pub fn split_map2_mut<U1, U2>(&mut self, f: impl Fn(&mut T) -> (&mut U1, &mut U2)) -> (MRx<U1>, MRx<U2>) {

    }

    pub fn split_map3_mut<U1, U2, U3>(&mut self, f: impl Fn(&mut T) -> (&mut U1, &mut U2, &mut U3)) -> (MRx<U1>, MRx<U2>, MRx<U3>) {

    }

    pub fn split_map4_mut<U1, U2, U3, U4>(&mut self, f: impl Fn(&mut T) -> (&mut U1, &mut U2, &mut U3, &mut U4)) -> (MRx<U1>, MRx<U2>, MRx<U3>, MRx<U4>) {

    }
}

impl <T, F: Fn(&Weak<dyn RxContext>) -> T> RxContext2 for RxContextImpl<T, F> {
    type T = T;

    fn replace(self: Rc<Self>, new_value: Self::T) -> Self::T {
        self.value.replace(new_value)
    }
}

impl <T, F: Fn(&Weak<dyn RxContext>) -> T> RxContext for RxContextImpl<T, F> {
    fn _recompute(self: Rc<Self>) {
        let computed = self.compute(&self);
        self.replace(computed);
    }
}

impl <T, F: Fn(&Weak<dyn RxContext>) -> T> RxContextImpl<T, F> {
    pub fn new(compute: F) -> Rc<Self> {
        Rc::new_cyclic(|this| RxContextImpl {
            value: RefCell::new(compute(this)),
            compute
        })
    }
}

/// Runs the function and re-runs every time one of its referenced dependencies changes.
pub fn with_rx<R>(f: impl Fn(&Weak<dyn RxContext>) -> R) -> IRx<R> {
    IRx::new(f)
}