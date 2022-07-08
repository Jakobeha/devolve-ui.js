//! `Rx` means "reactive value" (or "reactive X"). It is a wrapper for a value which changes,
//! and these changes trigger dependencies to re-run and change themselves. You can't access the
//! value directly, instead you use an associated function like `run_rx` to access it in a closure
//! which can re-run whenever the dependency changes. You can create new `Rx`s from old ones.

pub mod context;
pub mod observers;
pub mod run_rx;

use std::cell::{Ref, RefCell};
use std::collections::HashSet;
use std::hash::Hash;
use std::ops::{Deref, DerefMut, Drop};
use std::rc::{Rc, Weak};
use derive_more::{Deref, DerefMut};
use crate::core::data::rx::context::{AsRxContext, RxContext, RxContextRef};
use crate::core::data::rx::observers::RxObservers;
use crate::core::data::rx::run_rx::RunRxContext;
use test_log::test;

pub trait Rx<T> {
    type Ref<'a>: Deref<Target = T>;

    fn get(&self, c: &dyn AsRxContext) -> Self::Ref<'_>;

    fn map<'a, U>(&'a self, f: impl Fn(&'a T) -> U) -> CRx<U>
        where
            Self: Sized,
    {
        CRx::new(|c| f(self.get(c)))
    }

    fn split_map2<'a, U1, U2>(
        &'a self,
        f: impl Fn(&'a T) -> (U1, U2)
    ) -> (CRx<U1>, CRx<U2>)
        where
            Self: Sized,
    {
        (
            CRx::new(|c| f(self.get(c)).0),
            CRx::new(|c| f(self.get(c)).1),
        )
    }

    fn split_map3<'a, U1, U2, U3>(
        &'a self,
        f: impl Fn(&'a T) -> (U1, U2, U3),
    ) -> (CRx<U1>, CRx<U2>, CRx<U3>)
        where
            Self: Sized,
    {
        (
            CRx::new(|c| f(self.get(c)).0),
            CRx::new(|c| f(self.get(c)).1),
            CRx::new(|c| f(self.get(c)).2),
        )
    }

    fn split_map4<'a, U1, U2, U3, U4>(
        &'a self,
        f: impl Fn(&'a T) -> (U1, U2, U3, U4),
    ) -> (CRx<U1>, CRx<U2>, CRx<U3>, CRx<U4>)
        where
            Self: Sized,
    {
        (
            CRx::new(|c| f(self.get(c)).0),
            CRx::new(|c| f(self.get(c)).1),
            CRx::new(|c| f(self.get(c)).2),
            CRx::new(|c| f(self.get(c)).3),
        )
    }

    fn split_map5<'a, U1, U2, U3, U4, U5>(
        &'a self,
        f: impl Fn(&'a T) -> (U1, U2, U3, U4, U5),
    ) -> (CRx<U1>, CRx<U2>, CRx<U3>, CRx<U4>, CRx<U5>)
        where
            Self: Sized,
    {
        (
            CRx::new(|c| f(self.get(c)).0),
            CRx::new(|c| f(self.get(c)).1),
            CRx::new(|c| f(self.get(c)).2),
            CRx::new(|c| f(self.get(c)).3),
            CRx::new(|c| f(self.get(c)).4),
        )
    }
}

pub trait MRx<T>: Rx<T> {
    type RefMut<'a>: DerefMut<Target = T>;

    fn get_mut(&mut self, c: &dyn AsRxContext) -> Self::RefMut<'_>;

    fn set(&mut self, new_value: T);
    fn modify(&mut self, f: impl Fn(&mut T)) where Self: Sized;

    fn map_mut<'a, U>(&'a mut self, f: impl Fn(&'a mut T) -> &mut U) -> DRx<'a, U>
    where
        Self: Sized;

    fn split_map_mut2<'a, U1, U2>(
        &'a mut self,
        f: impl Fn(&'a mut T) -> (&'a mut U1, &'a mut U2)
    ) -> (DRx<'a, U1>, DRx<'a, U2>)
    where
        Self: Sized;

    fn split_map_mut3<'a, U1, U2, U3>(
        &'a mut self,
        f: impl Fn(&'a mut T) -> (&'a mut U1, &'a mut U2, &'a mut U3),
    ) -> (DRx<'a, U1>, DRx<'a, U2>, DRx<'a, U3>)
    where
        Self: Sized;

    fn split_map_mut4<'a, U1, U2, U3, U4>(
        &'a mut self,
        f: impl Fn(&'a mut T) -> (&'a mut U1, &'a mut U2, &'a mut U3, &'a mut U4),
    ) -> (DRx<'a, U1>, DRx<'a, U2>, DRx<'a, U3>, DRx<'a, U4>)
    where
        Self: Sized;

    fn split_map_mut5<'a, U1, U2, U3, U4, U5>(
        &'a mut self,
        f: impl Fn(&'a mut T) -> (&'a mut U1, &'a mut U2, &'a mut U3, &'a mut U4, &'a mut U5),
    ) -> (DRx<'a, U1>, DRx<'a, U2>, DRx<'a, U3>, DRx<'a, U4>, DRx<'a, U5>)
    where
        Self: Sized;
}

/// Source `Rx`. This stores its value directly. As such it never has to be recomputed,
/// and you can modify the value. You can even set or modify without a context, although the
/// modification will be rerun every time the value changes.
pub struct SRx<T> {
    value: T,
    observers: RxObservers
}

/// Computed `Rx`. This stores its value by running a closure and observing other `Rx`es requested
/// within the closure. You can't modify the value directly as it is computed.
pub struct CRx<T>(Rc<dyn IRxImplDyn<T = T>>);

/// Derived mutable `Rx`. This is a reference to data in an `SRx`. It forwards observers to the source.
/// This type allows you to split up a `SRx` and mutably borrow its parts at the same time.
pub struct DRx<'a, T> {
    value: &'a mut T,
    observers: &'a RxObservers
}

/// Derived computed `Rx`, which is actually just a computed `Rx` of a reference.
pub type DCRx<'a, T> = CRx<&'a T>;

trait IRxImplDyn: RxContext {
    type T;

    fn get(&self, c: &dyn AsRxContext) -> Ref<'_, Self::T>;
}

struct IRxImplImpl<T, F: Fn(&RxContextRef) -> T> {
    value: RefCell<T>,
    compute: F,
    observers: RxObservers,
}

/// Reference to data in an `Rx` which triggers update when it gets dropped.
pub struct MRxRef<'a, T, R>(&'a mut R);

trait _MRx<T>: Rx<T> {
    fn get_raw(&self) -> &T;
    fn get_mut_raw(&mut self) -> &mut T;
    fn observers(&self) -> &RxObservers;

    fn trigger(&self);
}

impl<T> Rx<T> for CRx<T> {
    type Ref<'a> = Ref<'a, T>;

    fn get(&self, c: &dyn AsRxContext) -> Self::Ref<'_> {
        self.0.get(c)
    }
}

impl<T> Rx<T> for SRx<T> {
    type Ref<'a> = &'a T;

    fn get(&self, _c: &dyn AsRxContext) -> Self::Ref<'_> {
        &self.value
    }
}

impl<T> Rx<T> for DRx<T> {
    type Ref<'a> = &'a T;

    fn get(&self, _c: &dyn AsRxContext) -> Self::Ref<'_> {
        self.value
    }
}

impl<T, R: _MRx<T>> MRx<T> for R {
    type RefMut<'a> = MRxRef<'a, T, Self>;

    fn get_mut(&mut self, c: &dyn AsRxContext) -> Self::RefMut<'_> {
        self.observers().insert(c.as_rx_context());
        MRxRef(self)
    }

    fn set(&mut self, new_value: T) {
        *self.get_mut_raw() = new_value;
        self.trigger();
    }

    fn modify(&mut self, f: impl Fn(&mut T)) {
        f(self.get_mut_raw());
        self.trigger();
        // Need to add observer after the trigger so that we don't re-trigger and recurse
        self.observers().insert(RxContextRef::owned(RunRxContext(|c| {
            // equivalent to `f(self.get_mut(c).deref_mut())`, except we don't trigger
            self.observers().insert(c.as_rx_context());
            f(self.get_mut_raw());
        })));
    }

    fn map_mut<'a, U>(&'a mut self, f: impl Fn(&'a mut T) -> &'a mut U) -> DRx<'a, U> {
        DRx {
            value: f(self.get_mut_raw()),
            observers: self.observers(),
        }
    }

    fn split_map_mut2<'a, U1, U2>(
        &'a mut self,
        f: impl Fn(&'a mut T) -> (&'a mut U1, &'a mut U2)
    ) -> (DRx<'a, U1>, DRx<'a, U2>) where Self: Sized {
        let (a, b) = f(self.get_mut_raw());
        (
            DRx { value: a, observers: self.observers() },
            DRx { value: b, observers: self.observers() }
        )
    }

    fn split_map_mut3<'a, U1, U2, U3>(
        &'a mut self,
        f: impl Fn(&'a mut T) -> (&'a mut U1, &'a mut U2, &'a mut U3)
    ) -> (DRx<'a, U1>, DRx<'a, U2>, DRx<'a, U3>) where Self: Sized {
        let (a, b, c) = f(self.get_mut_raw());
        (
            DRx { value: a, observers: self.observers() },
            DRx { value: b, observers: self.observers() },
            DRx { value: c, observers: self.observers() }
        )
    }

    fn split_map_mut4<'a, U1, U2, U3, U4>(
        &'a mut self,
        f: impl Fn(&'a mut T) -> (&'a mut U1, &'a mut U2, &'a mut U3, &'a mut U4)
    ) -> (DRx<'a, U1>, DRx<'a, U2>, DRx<'a, U3>, DRx<'a, U4>) where Self: Sized {
        let (a, b, c, d) = f(self.get_mut_raw());
        (
            DRx { value: a, observers: self.observers() },
            DRx { value: b, observers: self.observers() },
            DRx { value: c, observers: self.observers() },
            DRx { value: d, observers: self.observers() }
        )
    }

    fn split_map_mut5<'a, U1, U2, U3, U4, U5>(
        &'a mut self,
        f: impl Fn(&'a mut T) -> (&'a mut U1, &'a mut U2, &'a mut U3, &'a mut U4, &'a mut U5)
    ) -> (DRx<'a, U1>, DRx<'a, U2>, DRx<'a, U3>, DRx<'a, U4>, DRx<'a, U5>) where Self: Sized {
        let (a, b, c, d, e) = f(self.get_mut_raw());
        (
            DRx { value: a, observers: self.observers() },
            DRx { value: b, observers: self.observers() },
            DRx { value: c, observers: self.observers() },
            DRx { value: d, observers: self.observers() },
            DRx { value: e, observers: self.observers() }
        )
    }
}

impl<T> _MRx<T> for SRx<T> {
    fn get_raw(&self) -> &T {
        &self.value
    }

    fn get_mut_raw(&mut self) -> &mut T {
        &mut self.value
    }

    fn observers(&self) -> &RxObservers {
        &self.observers
    }

    fn trigger(&self) {
        self.observers.trigger()
    }
}

impl<'a, T> _MRx<T> for DRx<'a, T> {
    fn get_raw(&self) -> &T {
        &self.value
    }

    fn get_mut_raw(&mut self) -> &mut T {
        &mut self.value
    }

    fn observers(&self) -> &RxObservers {
        &self.observers
    }

    fn trigger(&self) {
        self.observers.trigger()
    }
}

impl<T> CRx<T> {
    pub fn new(compute: impl Fn(&RxContextRef) -> T) -> Self {
        CRx(IRxImplImpl::new(compute))
    }

    fn recompute(&self) {
        self.value.recompute();
        self.observers.trigger()
    }
}

impl<T> SRx<T> {
    pub fn new(value: T) -> Self {
        SRx {
            value,
            observers: RxObservers::new(),
        }
    }

    pub fn into_inner(self) -> T {
        self.value
    }
}


impl<'a, T, R: _MRx<T>> Deref for MRxRef<'a, T, R> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0.get_raw()
    }
}

impl<'a, T, R: _MRx<T>> DerefMut for MRxRef<'a, T, R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.get_mut_raw()
    }
}

impl<'a, T, R: _MRx<T>> Drop for MRxRef<'a, T, R> {
    fn drop(&mut self) {
        self.0.trigger();
    }
}

impl<T, F: Fn(&RxContextRef) -> T> IRxImplImpl<T, F> {
    fn recompute_without_trigger(&self) {
        let computed = self.compute(&self);
        self.value.replace(computed);
    }
}

impl<T, F: Fn(&RxContextRef) -> T> IRxImplDyn for IRxImplImpl<T, F> {
    type T = T;

    fn get(&self, c: &dyn AsRxContext) -> Ref<'_, Self::T> {
        self.observers.insert(c.as_rx_context());
        self.value.borrow()
    }
}

impl<T, F: Fn(&RxContextRef) -> T> RxContext for IRxImplImpl<T, F> {
    fn recompute(&self) {
        self.recompute_without_trigger();
        self.observers.trigger();
    }
}

impl<T, F: Fn(&RxContextRef) -> T> IRxImplImpl<T, F> {
    pub fn new(compute: F) -> Rc<Self> {
        Rc::new_cyclic(|this| IRxImplImpl {
            // have to clone because RxContext :(
            value: RefCell::new(compute(&RxContextRef::Weak(this.clone()))),
            compute,
            observers: RxObservers::new()
        })
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_rxs() {
        let mut rx = SRx::new(vec![1, 2, 3]);
        assert_eq!(rx.get(), vec![1, 2, 3]);
        rx.set(vec![1, 2, 4]);
        assert_eq!(rx.get(), vec![1, 2, 4]);
        rx.set(vec![1, 2, 5]);
        assert_eq!(rx.get(), vec![1, 2, 5]);

        {
            let mut drx = rx.map_mut(|x| x.get_mut(0).unwrap());
            assert_eq!(drx.get(), &1);
            drx.set(2);
            assert_eq!(drx.get(), &2);
        }
        assert_eq!(rx.get(), vec![2, 2, 5]);

        {
            let (mut drx0, mut drx1, mut drx2) = rx.split_map_mut3(|x| (x.get_mut(0).unwrap(), x.get_mut(1).unwrap(), x.get_mut(2).unwrap()));
            assert_eq!(drx0.get(), &1);
            assert_eq!(drx1.get(), &2);
            assert_eq!(drx2.get(), &5);
            drx0.set(2);
            drx1.set(3);
            drx2.set(4);
        }
        assert_eq!(rx.get(), vec![2, 3, 4]);

        let mut crx = CRx::new(|c| rx.get(c)[0] * 2);
        let mut crx2 = CRx::new(|c| crx.get(c) + rx.get(c)[1] * 10);
        let mut crx3 = crx.map(|x| x.to_string());
        assert_eq!(crx.get(), 2);
        assert_eq!(crx2.get(), 22);
        assert_eq!(crx3.get(), "2");
        rx.set(vec![2, 3, 4]);
        assert_eq!(crx.get(), 4);
        assert_eq!(crx2.get(), 34);
        assert_eq!(crx3.get(), "4");
        rx.set(vec![3, 4, 5]);
        assert_eq!(crx.get(), 6);
        assert_eq!(crx2.get(), 46);
        assert_eq!(crx3.get(), "6");
    }
}