//! `Rx` means "reactive value" (or "reactive X"). It is a wrapper for a value which changes,
//! and these changes trigger dependencies to re-run and change themselves. You can't access the
//! value directly, instead you use an associated function like `with_rx` to access it in a closure
//! which can re-run whenever the dependency changes.

use std::cell::RefCell;
use std::ops::{Deref, DerefMut, Drop};
use std::rc::{Rc, Weak};

pub trait Rx<T> {
    fn get(&self, c: &dyn AsRxContext) -> &T;

    fn map<'a, U>(&'a self, f: impl Fn(&'a T) -> U) -> IRx<U>
    where
        Self: Sized,
    {
        IRx::new(|c| f(self.get(c)))
    }

    fn split_map2<'a, U1, U2>(&'a self, f: impl Fn(&'a T) -> (U1, U2)) -> (IRx<U1>, IRx<U2>)
    where
        Self: Sized,
    {
        (
            IRx::new(|c| f(self.get(c)).0),
            IRx::new(|c| f(self.get(c)).1),
        )
    }

    fn split_map3<'a, U1, U2, U3>(
        &'a self,
        f: impl Fn(&'a T) -> (U1, U2, U3),
    ) -> (IRx<U1>, IRx<U2>, IRx<U3>)
    where
        Self: Sized,
    {
        (
            IRx::new(|c| f(self.get(c)).0),
            IRx::new(|c| f(self.get(c)).1),
            IRx::new(|c| f(self.get(c)).2),
        )
    }

    fn split_map4<'a, U1, U2, U3, U4>(
        &'a self,
        f: impl Fn(&'a T) -> (U1, U2, U3, U4),
    ) -> (IRx<U1>, IRx<U2>, IRx<U3>, IRx<U4>)
    where
        Self: Sized,
    {
        (
            IRx::new(|c| f(self.get(c)).0),
            IRx::new(|c| f(self.get(c)).1),
            IRx::new(|c| f(self.get(c)).2),
            IRx::new(|c| f(self.get(c)).3),
        )
    }

    fn split_map5<'a, U1, U2, U3, U4, U5>(
        &'a self,
        f: impl Fn(&'a T) -> (U1, U2, U3, U4, U5),
    ) -> (IRx<U1>, IRx<U2>, IRx<U3>, IRx<U4>, IRx<U5>)
    where
        Self: Sized,
    {
        (
            IRx::new(|c| f(self.get(c)).0),
            IRx::new(|c| f(self.get(c)).1),
            IRx::new(|c| f(self.get(c)).2),
            IRx::new(|c| f(self.get(c)).3),
            IRx::new(|c| f(self.get(c)).4),
        )
    }
}

pub struct MRx<T> {
    value: T,
    observers: RxObservers,
}

pub struct IRx<T> {
    value: Rc<dyn RxContext2<T = T>>,
    observers: RxObservers,
}

pub type DMRx<'a, T> = MRx<&'a mut T>;
pub type DIRx<'a, T> = IRx<&'a T>;

struct RxObservers(RefCell<HashSet<WeakRxContext>>);

#[derive(Deref, DerefMut)]
pub struct WeakRxContext(Weak<dyn RxContext>);

impl Eq for WeakRxContext {
    fn eq(lhs: &Self, rhs)
    }
}

pub trait AsRxContext {
    fn as_rx_context(&self) -> WeakRxContext;
}

impl AsRxContext for WeakRxContext {
    fn as_rx_context(&self) -> WeakRxContext {
        self.clone()
    }
}

pub trait RxContext {
    fn _recompute(self: Rc<Self>);
}

// Dumb dynamic object restriction
impl dyn RxContext {
    pub fn recompute(self: &Rc<Self>) {
        self.clone()._recompute()
    }
}

impl<T> dyn RxContext2<T = T> {
    pub fn recompute(self: &Rc<Self>) {
        self.clone()._recompute()
    }

    pub fn _get(self: Rc<Self>) -> &T {
        self.clone().get()
    }

    pub fn _replace(self: Rc<Self>, new_value: T) -> T {
        self.clone().replace(new_value)
    }
}

pub trait RxContext2: RxContext {
    type T;

    fn _get(self: Rc<Self>) -> &Self::T;
    fn _replace(self: Rc<Self>, new_value: Self::T) -> Self::T;
}

pub struct RxContextImpl<T, F: Fn(&WeakRxContext) -> T> {
    value: RefCell<T>,
    compute: F,
}

pub struct MRxRef<'a, T>(&'a mut MRx<T>);

impl<T> Rx<T> for IRx<T> {
    fn get(&self, c: &dyn AsRxContext) -> &T {
        self.observers.borrow_mut().push(c.as_rx_context());
        &self.value.get()
    }
}

impl<T> IRx<T> {
    pub fn new(compute: impl Fn(&WeakRxContext) -> T) -> Self {
        IRx {
            value: RxContextImpl::new(compute),
            observers: RefCell::new(Vec::new()),
        }
    }

    fn recompute(&self) {
        self.value.recompute();
        for observer in self.observers.borrow().iter() {
            if let Some(observer) = observer.upgrade() {
                observer.recompute();
            }
        }
    }
}

impl<T> MRx<T> {
    pub fn new(initial: T) -> Self {
        MRx {
            value: initial,
            observers: RefCell::new(Vec::new()),
        }
    }

    pub fn get(&self) -> &T {
        &self.value
    }

    pub fn get_mut(&mut self) -> MRxRef<'_, T> {
        MRxRef(&mut self)
    }

    pub fn set(&mut self, new_value: T) {
        self.value = new_value;
        self.recompute();
    }

    pub fn into_inner(self) -> T {
        self.value
    }

    fn recompute(&self) {
        for observer in self.observers.borrow().iter() {
            if let Some(observer) = observer.upgrade() {
                observer.recompute();
            }
        }
    }
}

impl<'a, T> Deref for MRxRef<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0.value
    }
}

impl<'a, T> DerefMut for MRxRef<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0.value
    }
}

impl<'a, T> Drop for MRxRef<'a, T> {
    fn drop(&mut self) {
        self.0.recompute();
    }
}

impl<T, F: Fn(&WeakRxContext) -> T> RxContext2 for RxContextImpl<T, F> {
    type T = T;

    fn _get(self: Rc<Self>) -> Ref<'_, Self::T> {
        self.value.borrow()
    }

    fn _replace(self: Rc<Self>, new_value: Self::T) -> Self::T {
        self.value.replace(new_value)
    }
}

impl<T, F: Fn(&WeakRxContext) -> T> RxContext for RxContextImpl<T, F> {
    fn _recompute(self: Rc<Self>) {
        let computed = self.compute(&self);
        self.replace(computed);
    }
}

impl<T, F: Fn(&WeakRxContext) -> T> RxContextImpl<T, F> {
    pub fn new(compute: F) -> Rc<Self> {
        Rc::new_cyclic(|this| RxContextImpl {
            value: RefCell::new(compute(this)),
            compute,
        })
    }
}

/// Runs the function and re-runs every time one of its referenced dependencies changes.
pub fn with_rx<R>(f: impl Fn(&WeakRxContext) -> R) -> IRx<R> {
    IRx::new(f)
}
