//! `Rx` means "reactive value" (or "reactive X"). It is a wrapper for a value which changes,
//! and these changes trigger dependencies to re-run and change themselves. You can't access the
//! value directly, instead you use an associated function like `run_rx` to access it in a closure
//! which can re-run whenever the dependency changes. You can create new `Rx`s from old ones.

pub mod context;
pub mod observers;
pub mod run_rx;

use std::cell::{Ref, RefCell};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Drop};
use std::rc::Rc;
use crate::core::data::rx::context::{AsRxContext, RxContext, RxContextRef};
use crate::core::data::rx::observers::RxObservers;
use crate::core::data::rx::run_rx::RunRxContext;

pub trait Rx<T> {
    type Ref<'a>: Deref<Target = T> where Self: 'a, T: 'a;

    fn get(&self, c: &dyn AsRxContext) -> Self::Ref<'_>;

    fn map<'a, U>(&'a self, f: impl Fn(&T) -> U) -> CRx<U>
        where
            Self: Sized,
            T: 'a
    {
        CRx::new(|c| f(self.get(c).deref()))
    }

    fn split_map2<'a, U1, U2>(
        &'a self,
        f: impl Fn(&T) -> (U1, U2)
    ) -> (CRx<U1>, CRx<U2>)
        where
            Self: Sized,
            T: 'a
    {
        (
            CRx::new(|c| f(self.get(c).deref()).0),
            CRx::new(|c| f(self.get(c).deref()).1),
        )
    }

    fn split_map3<'a, U1, U2, U3>(
        &'a self,
        f: impl Fn(&T) -> (U1, U2, U3),
    ) -> (CRx<U1>, CRx<U2>, CRx<U3>)
        where
            Self: Sized,
            T: 'a
    {
        (
            CRx::new(|c| f(self.get(c).deref()).0),
            CRx::new(|c| f(self.get(c).deref()).1),
            CRx::new(|c| f(self.get(c).deref()).2),
        )
    }

    fn split_map4<'a, U1, U2, U3, U4>(
        &'a self,
        f: impl Fn(&T) -> (U1, U2, U3, U4),
    ) -> (CRx<U1>, CRx<U2>, CRx<U3>, CRx<U4>)
        where
            Self: Sized,
            T: 'a
    {
        (
            CRx::new(|c| f(self.get(c).deref()).0),
            CRx::new(|c| f(self.get(c).deref()).1),
            CRx::new(|c| f(self.get(c).deref()).2),
            CRx::new(|c| f(self.get(c).deref()).3),
        )
    }

    fn split_map5<'a, U1, U2, U3, U4, U5>(
        &'a self,
        f: impl Fn(&T) -> (U1, U2, U3, U4, U5),
    ) -> (CRx<U1>, CRx<U2>, CRx<U3>, CRx<U4>, CRx<U5>)
        where
            Self: Sized,
            T: 'a
    {
        (
            CRx::new(|c| f(self.get(c).deref()).0),
            CRx::new(|c| f(self.get(c).deref()).1),
            CRx::new(|c| f(self.get(c).deref()).2),
            CRx::new(|c| f(self.get(c).deref()).3),
            CRx::new(|c| f(self.get(c).deref()).4),
        )
    }
}

pub trait MRx<T>: Rx<T> {
    type RefMut<'a>: DerefMut<Target = T> where Self: 'a, T: 'a;

    fn get_mut<'a>(&'a mut self, c: &dyn AsRxContext) -> Self::RefMut<'a> where T: 'a;

    fn set(&mut self, new_value: T);
    fn modify(&mut self, f: impl Fn(&mut T)) where Self: Sized;

    fn map_mut<'a, U>(&'a mut self, f: impl Fn(&'a mut T) -> &mut U) -> DRx<'a, U>
    where
        Self: Sized,
        T: 'a;

    fn split_map_mut2<'a, U1, U2>(
        &'a mut self,
        f: impl Fn(&'a mut T) -> (&'a mut U1, &'a mut U2)
    ) -> (DRx<'a, U1>, DRx<'a, U2>)
    where
        Self: Sized,
        T: 'a;

    fn split_map_mut3<'a, U1, U2, U3>(
        &'a mut self,
        f: impl Fn(&'a mut T) -> (&'a mut U1, &'a mut U2, &'a mut U3),
    ) -> (DRx<'a, U1>, DRx<'a, U2>, DRx<'a, U3>)
    where
        Self: Sized,
        T: 'a;

    fn split_map_mut4<'a, U1, U2, U3, U4>(
        &'a mut self,
        f: impl Fn(&'a mut T) -> (&'a mut U1, &'a mut U2, &'a mut U3, &'a mut U4),
    ) -> (DRx<'a, U1>, DRx<'a, U2>, DRx<'a, U3>, DRx<'a, U4>)
    where
        Self: Sized,
        T: 'a;

    fn split_map_mut5<'a, U1, U2, U3, U4, U5>(
        &'a mut self,
        f: impl Fn(&'a mut T) -> (&'a mut U1, &'a mut U2, &'a mut U3, &'a mut U4, &'a mut U5),
    ) -> (DRx<'a, U1>, DRx<'a, U2>, DRx<'a, U3>, DRx<'a, U4>, DRx<'a, U5>)
    where
        Self: Sized,
        T: 'a;
}

/// Source `Rx`. This stores its value directly. As such it never has to be recomputed,
/// and you can modify the value. You can even set or modify without a context, although the
/// modification will be rerun every time the value changes.
pub struct SRx<'a, T: 'a> {
    value: T,
    observers: RxObservers<'a>
}

/// Computed `Rx`. This stores its value by running a closure and observing other `Rx`es requested
/// within the closure. You can't modify the value directly as it is computed.
pub struct CRx<'a, T>(Rc<dyn CRxImplDyn<T = T> + 'a>);

/// Derived mutable `Rx`. This is a reference to data in an `SRx`. It forwards observers to the source.
/// This type allows you to split up a `SRx` and mutably borrow its parts at the same time.
pub struct DRx<'a, 'b: 'a, T: 'b> {
    value: &'a mut T,
    observers: &'a RxObservers<'b>
}

/// Derived computed `Rx`, which is actually just a computed `Rx` of a reference.
pub type DCRx<'a, 'b, T> = CRx<'b, &'a T>;

trait CRxImplDyn<'a>: RxContext {
    type T;

    fn get(&self, c: &dyn AsRxContext) -> Ref<'_, Self::T>;
}

struct CRxImplImpl<'a, T: 'a, F: FnMut(&RxContextRef) -> T + 'a> {
    value: RefCell<T>,
    compute: RefCell<F>,
    observers: RxObservers<'a>,
}

/// Reference to data in an `Rx` which triggers update when it gets dropped.
pub struct MRxRef<'a, T, R: _MRx<T>>(&'a mut R, PhantomData<T>);

trait _MRx<T>: Rx<T> {
    fn get_raw(&self) -> &T;
    fn get_mut_raw(&mut self) -> &mut T;
    fn observers(&self) -> &RxObservers;
    fn observers_and_get_mut_raw(&mut self) -> (&RxObservers, &mut T);

    fn trigger(&self);
}

impl<'b, T: 'b> Rx<T> for CRx<'b, T> {
    type Ref<'a> = Ref<'a, T> where Self: 'a, T: 'a;

    fn get(&self, c: &dyn AsRxContext) -> Self::Ref<'_> {
        self.0.get(c)
    }
}

impl<T> Rx<T> for SRx<T> {
    type Ref<'a> = &'a T where Self: 'a, T: 'a;

    fn get(&self, _c: &dyn AsRxContext) -> Self::Ref<'_> {
        &self.value
    }
}

impl<'b, T> Rx<T> for DRx<'b, T> {
    type Ref<'a> = &'a T where Self: 'a, T: 'a;

    fn get(&self, _c: &dyn AsRxContext) -> Self::Ref<'_> {
        self.value
    }
}

impl<T, R: _MRx<T>> MRx<T> for R {
    type RefMut<'a> = MRxRef<'a, T, Self> where Self: 'a, T: 'a;

    fn get_mut<'a>(&'a mut self, c: &dyn AsRxContext) -> Self::RefMut<'a> where T: 'a {
        self.observers().insert(c.as_rx_context());
        MRxRef(self, PhantomData)
    }

    fn set(&mut self, new_value: T) {
        *self.get_mut_raw() = new_value;
        self.trigger();
    }

    fn modify(&mut self, f: impl Fn(&mut T)) {
        f(self.get_mut_raw());
        self.trigger();
        let (observers, mut_raw) = self.observers_and_get_mut_raw();
        // Need to add observer after the trigger so that we don't re-trigger and recurse
        observers.insert(RxContextRef::owned(RunRxContext::new(|c| {
            // equivalent to `f(self.get_mut(c).deref_mut())`, except we don't trigger
            observers.insert(c.as_rx_context());
            f(mut_raw);
        })));
    }

    fn map_mut<'a, U>(&'a mut self, f: impl Fn(&'a mut T) -> &'a mut U) -> DRx<'a, U> where T: 'a {
        let (observers, mut_raw) = self.observers_and_get_mut_raw();
        DRx {
            value: f(mut_raw),
            observers
        }
    }

    fn split_map_mut2<'a, U1, U2>(
        &'a mut self,
        f: impl Fn(&'a mut T) -> (&'a mut U1, &'a mut U2)
    ) -> (DRx<'a, U1>, DRx<'a, U2>) where T: 'a {
        let (observers, mut_raw) = self.observers_and_get_mut_raw();
        let (a, b) = f(mut_raw);
        (
            DRx { value: a, observers },
            DRx { value: b, observers }
        )
    }

    fn split_map_mut3<'a, U1, U2, U3>(
        &'a mut self,
        f: impl Fn(&'a mut T) -> (&'a mut U1, &'a mut U2, &'a mut U3)
    ) -> (DRx<'a, U1>, DRx<'a, U2>, DRx<'a, U3>) where T: 'a {
        let (observers, mut_raw) = self.observers_and_get_mut_raw();
        let (a, b, c) = f(mut_raw);
        (
            DRx { value: a, observers },
            DRx { value: b, observers },
            DRx { value: c, observers }
        )
    }

    fn split_map_mut4<'a, U1, U2, U3, U4>(
        &'a mut self,
        f: impl Fn(&'a mut T) -> (&'a mut U1, &'a mut U2, &'a mut U3, &'a mut U4)
    ) -> (DRx<'a, U1>, DRx<'a, U2>, DRx<'a, U3>, DRx<'a, U4>) where T: 'a {
        let (observers, mut_raw) = self.observers_and_get_mut_raw();
        let (a, b, c, d) = f(mut_raw);
        (
            DRx { value: a, observers },
            DRx { value: b, observers },
            DRx { value: c, observers },
            DRx { value: d, observers }
        )
    }

    fn split_map_mut5<'a, U1, U2, U3, U4, U5>(
        &'a mut self,
        f: impl Fn(&'a mut T) -> (&'a mut U1, &'a mut U2, &'a mut U3, &'a mut U4, &'a mut U5)
    ) -> (DRx<'a, U1>, DRx<'a, U2>, DRx<'a, U3>, DRx<'a, U4>, DRx<'a, U5>) where T: 'a {
        let (observers, mut_raw) = self.observers_and_get_mut_raw();
        let (a, b, c, d, e) = f(mut_raw);
        (
            DRx { value: a, observers },
            DRx { value: b, observers },
            DRx { value: c, observers },
            DRx { value: d, observers },
            DRx { value: e, observers }
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

    fn observers_and_get_mut_raw(&mut self) -> (&RxObservers, &mut T) {
        (&self.observers, &mut self.value)
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

    fn observers_and_get_mut_raw(&mut self) -> (&RxObservers, &mut T) {
        (&self.observers, &mut self.value)
    }

    fn trigger(&self) {
        self.observers.trigger()
    }
}

impl<T> CRx<T> {
    pub fn new(compute: impl FnMut(&RxContextRef) -> T) -> Self {
        CRx(CRxImplImpl::new(compute))
    }

    fn recompute(&self) {
        self.0.clone().recompute()
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

impl<T, F: FnMut(&RxContextRef) -> T> CRxImplImpl<T, F> {
    fn recompute_without_trigger(self: Rc<Self>) {
        let self_ = self.clone();
        match self.compute.try_borrow_mut() {
            Err(err) => {
                panic!("compute recursively caused compute: {}", err)
            }
            Ok(mut compute) => {
                let computed = compute(&RxContextRef::Strong(self_));
                self.value.replace(computed);
            }
        }
    }
}

impl<T, F: FnMut(&RxContextRef) -> T> CRxImplDyn for CRxImplImpl<T, F> {
    type T = T;

    fn get(&self, c: &dyn AsRxContext) -> Ref<'_, Self::T> {
        self.observers.insert(c.as_rx_context());
        self.value.borrow()
    }
}

impl<T, F: FnMut(&RxContextRef) -> T> RxContext for CRxImplImpl<T, F> {
    fn recompute(self: Rc<Self>) {
        self.clone().recompute_without_trigger();
        self.observers.trigger();
    }
}

impl<T, F: FnMut(&RxContextRef) -> T> CRxImplImpl<T, F> {
    pub fn new(mut compute: F) -> Rc<Self> {
        Rc::<CRxImplImpl<T, F>>::new_cyclic(|this| {
            let value = compute(&RxContextRef::Weak(this.clone()));
            CRxImplImpl {
                // have to clone because RxContext :(
                value: RefCell::new(value),
                compute: RefCell::new(compute),
                observers: RxObservers::new()
            }
        })
    }
}

#[cfg(test)]
pub mod tests {
    use test_log::test;
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