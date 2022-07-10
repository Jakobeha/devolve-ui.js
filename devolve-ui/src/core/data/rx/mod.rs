//! `Rx` means "reactive value" (or "reactive X"). It is a wrapper for a value which changes,
//! and these changes trigger dependencies to re-run and change themselves. You can't access the
//! value directly, instead you use an associated function like `run_rx` to access it in a closure
//! which can re-run whenever the dependency changes. You can create new `Rx`s from old ones.

pub mod context;
pub mod observers;
pub mod run_rx;
pub mod snapshot_ctx;
pub mod refs;

use std::cell::{Ref, RefCell};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use crate::core::data::rx::refs::{DRxRef, DRxRefCell, MRxRef, MRxRefCell, SRxRefCell};
use crate::core::data::rx::context::{AsRxContext, RxContext, RxContextRef};
use crate::core::data::rx::observers::RxObservers;
use crate::core::data::rx::run_rx::RunRxContext;

pub trait Rx<'c, T: 'c> {
    type Ref<'a>: Deref<Target = T> where Self: 'a, 'c: 'a;

    fn get<'a>(&'a self, c: &(dyn AsRxContext<'c> + 'c)) -> Self::Ref<'a> where 'c: 'a;

    fn map<'c2: 'c, U: 'c2>(&'c2 self, f: impl Fn(&T) -> U + 'c2) -> CRx<'c2, U> where Self: Sized, T: 'c2 {
        CRx::<'c2, U>::new(move |c| f(self.get(c).deref()))
    }

    fn split_map2<'c2: 'c, U1: 'c2, U2: 'c2>(
        &'c2 self,
        f: impl Fn(&T) -> (U1, U2) + 'c2
    ) -> (CRx<'c2, U1>, CRx<'c2, U2>) where Self: Sized, T: 'c2 {
        let f1 = Rc::new(move |c: &RxContextRef<'c2>| f(self.get(c).deref()));
        let f2 = f1.clone();
        (
            CRx::<'c2, U1>::new(move |c| f1(c).0),
            CRx::<'c2, U2>::new(move |c| f2(c).1),
        )
    }

    fn split_map3<'c2: 'c, U1: 'c2, U2: 'c2, U3: 'c2> (
        &'c2 self,
        f: impl Fn(&T) -> (U1, U2, U3) + 'c2,
    ) -> (CRx<'c2, U1>, CRx<'c2, U2>, CRx<'c2, U3>) where Self: Sized, T: 'c2 {
        let f1 = Rc::new(move |c: &RxContextRef<'c2>| f(self.get(c).deref()));
        let f2 = f1.clone();
        let f3 = f2.clone();
        (
            CRx::<'c2, U1>::new(move |c| f1(c).0),
            CRx::<'c2, U2>::new(move |c| f2(c).1),
            CRx::<'c2, U3>::new(move |c| f3(c).2),
        )
    }

    fn split_map4<'c2: 'c, U1: 'c2, U2: 'c2, U3: 'c2, U4: 'c2>(
        &'c2 self,
        f: impl Fn(&T) -> (U1, U2, U3, U4) + 'c2,
    ) -> (CRx<'c2, U1>, CRx<'c2, U2>, CRx<'c2, U3>, CRx<'c2, U4>) where Self: Sized, T: 'c2 {
        let f1 = Rc::new(move |c: &RxContextRef<'c2>| f(self.get(c).deref()));
        let f2 = f1.clone();
        let f3 = f2.clone();
        let f4 = f3.clone();
        (
            CRx::<'c2, U1>::new(move |c| f1(c).0),
            CRx::<'c2, U2>::new(move |c| f2(c).1),
            CRx::<'c2, U3>::new(move |c| f3(c).2),
            CRx::<'c2, U4>::new(move |c| f4(c).3)
        )
    }

    fn split_map5<'c2: 'c, U1: 'c2, U2: 'c2, U3: 'c2, U4: 'c2, U5: 'c2>(
        &'c2 self,
        f: impl Fn(&T) -> (U1, U2, U3, U4, U5) + 'c2,
    ) -> (CRx<'c2, U1>, CRx<'c2, U2>, CRx<'c2, U3>, CRx<'c2, U4>, CRx<'c2, U5>) where Self: Sized, T: 'c2 {
        let f1 = Rc::new(move |c: &RxContextRef<'c2>| f(self.get(c).deref()));
        let f2 = f1.clone();
        let f3 = f2.clone();
        let f4 = f3.clone();
        let f5 = f4.clone();
        (
            CRx::<'c2, U1>::new(move |c| f1(c).0),
            CRx::<'c2, U2>::new(move |c| f2(c).1),
            CRx::<'c2, U3>::new(move |c| f3(c).2),
            CRx::<'c2, U4>::new(move |c| f4(c).3),
            CRx::<'c2, U5>::new(move |c| f5(c).4),
        )
    }
}

pub trait MRx<'c, T: 'c>: Rx<'c, T> {
    type RefMut<'a>: DerefMut<Target = T> where Self: 'a, 'c: 'a;

    fn get_mut<'a>(&'a mut self, c: &(dyn AsRxContext<'c> + 'c)) -> Self::RefMut<'a> where 'c: 'a;

    fn set(&self, new_value: T);
    fn modify(&'c self, f: impl Fn(&mut T) + 'c) where Self: Sized;

    fn map_mut<'a, U>(&'a self, f: impl Fn(&mut T) -> &mut U + 'c) -> DRx<'a, 'c, U> where Self: Sized, 'c: 'a;

    fn split_map_mut2<'a, U1, U2>(
        &'a self,
        f: impl Fn(&mut T) -> (&mut U1, &mut U2) + 'c
    ) -> (DRx<'a, 'c, U1>, DRx<'a, 'c, U2>) where Self: Sized, 'c: 'a;

    fn split_map_mut3<'a, U1, U2, U3>(
        &'a self,
        f: impl Fn(&mut T) -> (&mut U1, &mut U2, &mut U3) + 'c
    ) -> (DRx<'a, 'c, U1>, DRx<'a, 'c, U2>, DRx<'a, 'c, U3>) where Self: Sized, 'c: 'a;

    fn split_map_mut4<'a, U1, U2, U3, U4>(
        &'a self,
        f: impl Fn(&mut T) -> (&mut U1, &mut U2, &mut U3, &mut U4) + 'c
    ) -> (DRx<'a, 'c, U1>, DRx<'a, 'c, U2>, DRx<'a, 'c, U3>, DRx<'a, 'c, U4>) where Self: Sized, 'c: 'a;

    fn split_map_mut5<'a, U1, U2, U3, U4, U5>(
        &'a self,
        f: impl Fn(&mut T) -> (&mut U1, &mut U2, &mut U3, &mut U4, &mut U5) + 'c
    ) -> (DRx<'a, 'c, U1>, DRx<'a, 'c, U2>, DRx<'a, 'c, U3>, DRx<'a, 'c, U4>, DRx<'a, 'c, U5>) where Self: Sized, 'c: 'a;
}

/// Source `Rx`. This stores its value directly. As such it never has to be recomputed,
/// and you can modify the value. You can even set or modify without a context, although the
/// modification will be rerun every time the value changes.
pub struct SRx<'c, T: 'c> {
    value: RefCell<T>,
    observers: RxObservers<'c>
}

/// Computed `Rx`. This stores its value by running a closure and observing other `Rx`es requested
/// within the closure. You can't modify the value directly as it is computed.
pub struct CRx<'c, T>(Rc<dyn CRxImplDyn<'c, T = T> + 'c>);

/// Derived mutable `Rx`. This is a reference to data in an `SRx`. It forwards observers to the source.
/// This type allows you to split up a `SRx` and mutably borrow its parts at the same time.
pub struct DRx<'a, 'c: 'a, T: 'c> {
    value: RefCell<&'a mut T>,
    observers: &'a RxObservers<'c>
}

/// Derived computed `Rx`, which is actually just a computed `Rx` of a reference.
pub type DCRx<'a, 'c, T> = CRx<'c, &'a T>;

trait CRxImplDyn<'c>: RxContext {
    type T;

    fn get<'a>(&'a self, c: &(dyn AsRxContext<'c> + 'c)) -> Ref<'a, Self::T>;
}

struct CRxImplImpl<'c, T: 'c, F: FnMut(&RxContextRef<'c>) -> T + 'c> {
    value: RefCell<T>,
    compute: RefCell<F>,
    observers: RxObservers<'c>,
}

pub(super) trait _MRx<'a, 'c: 'a, T: 'c>: Rx<'c, T> {
    type RawRef<'b>: MRxRefCell<'b, T> where Self: 'b, 'a: 'b;

    fn get_raw(&self) -> Self::RawRef<'_>;
    fn observers(&self) -> &RxObservers<'c>;
    fn observers_and_get_raw(&self) -> (&RxObservers<'c>, Self::RawRef<'_>);

    fn trigger(&self);
}

impl<'c, T: 'c> Rx<'c, T> for CRx<'c, T> {
    type Ref<'a> = Ref<'a, T> where Self: 'a, 'c: 'a;

    fn get<'a>(&'a self, c: &(dyn AsRxContext<'c> + 'c)) -> Self::Ref<'a> where 'c: 'a {
        self.0.get(c)
    }
}

impl<'c, T: 'c> Rx<'c, T> for SRx<'c, T> {
    type Ref<'a> = Ref<'a, T> where Self: 'a, 'c: 'a;

    fn get<'a>(&'a self, c: &(dyn AsRxContext<'c> + 'c)) -> Self::Ref<'a> where 'c: 'a {
        self.observers.insert(c.as_rx_context());
        self.value.borrow()
    }
}

impl<'a, 'c: 'a, T: 'c> Rx<'c, T> for DRx<'a, 'c, T> {
    type Ref<'b> = DRxRef<'b, 'a, T> where Self: 'b, 'c: 'b;

    fn get<'b>(&'b self, c: &(dyn AsRxContext<'c> + 'c)) -> Self::Ref<'b> where 'c: 'b {
        self.observers.insert(c.as_rx_context());
        DRxRef(self.value.borrow())
    }
}

impl<'a, 'c: 'a, T: 'c, R: _MRx<'a, 'c, T>> MRx<'c, T> for R {
    type RefMut<'b> = MRxRef<'b, 'c, T, Self> where Self: 'b, 'c: 'b;

    fn get_mut<'b>(&'b mut self, c: &(dyn AsRxContext<'c> + 'c)) -> Self::RefMut<'b> where 'c: 'b {
        self.observers().insert(c.as_rx_context());
        MRxRef(self, PhantomData)
    }

    fn set(&self, new_value: T) {
        self.get_raw().replace(new_value);
        self.trigger();
    }

    fn modify(&'c self, f: impl Fn(&mut T) + 'c) {
        f(&mut *self.get_raw().borrow_mut());
        self.trigger();
        let (observers, raw) = self.observers_and_get_raw();
        // Need to add observer after the trigger so that we don't re-trigger and recurse
        observers.insert(RxContextRef::owned(RunRxContext::<'c, _>::new(move |c| {
            // equivalent to `f(self.get_mut(c).deref_mut())`, except we don't trigger
            observers.insert(c.as_rx_context());
            f(&mut *raw.borrow_mut());
        })));
    }

    fn map_mut<'b, U>(&'b self, f: impl Fn(&mut T) -> &mut U + 'c) -> DRx<'b, 'c, U> where 'c: 'b {
        let (observers, raw) = self.observers_and_get_raw();
        DRx {
            value: RefCell::new(f(&mut *raw.borrow_mut())),
            observers
        }
    }

    fn split_map_mut2<'b, U1, U2>(
        &'b self,
        f: impl Fn(&mut T) -> (&mut U1, &mut U2) + 'c
    ) -> (DRx<'b, 'c, U1>, DRx<'b, 'c, U2>) where 'c: 'b {
        let (observers, raw) = self.observers_and_get_raw();
        let (a, b) = f(&mut *raw.borrow_mut());
        (
            DRx { value: RefCell::new(a), observers },
            DRx { value: RefCell::new(b), observers }
        )
    }

    fn split_map_mut3<'b, U1, U2, U3>(
        &'b self,
        f: impl Fn(&mut T) -> (&mut U1, &mut U2, &mut U3) + 'c
    ) -> (DRx<'b, 'c, U1>, DRx<'b, 'c, U2>, DRx<'b, 'c, U3>) where 'c: 'b {
        let (observers, raw) = self.observers_and_get_raw();
        let (a, b, c) = f(&mut *raw.borrow_mut());
        (
            DRx { value: RefCell::new(a), observers },
            DRx { value: RefCell::new(b), observers },
            DRx { value: RefCell::new(c), observers }
        )
    }

    fn split_map_mut4<'b, U1, U2, U3, U4>(
        &'b self,
        f: impl Fn(&mut T) -> (&mut U1, &mut U2, &mut U3, &mut U4) + 'c
    ) -> (DRx<'b, 'c, U1>, DRx<'b, 'c, U2>, DRx<'b, 'c, U3>, DRx<'b, 'c, U4>) where 'c: 'b {
        let (observers, raw) = self.observers_and_get_raw();
        let (a, b, c, d) = f(&mut *raw.borrow_mut());
        (
            DRx { value: RefCell::new(a), observers },
            DRx { value: RefCell::new(b), observers },
            DRx { value: RefCell::new(c), observers },
            DRx { value: RefCell::new(d), observers }
        )
    }

    fn split_map_mut5<'b, U1, U2, U3, U4, U5>(
        &'b self,
        f: impl Fn(&mut T) -> (&mut U1, &mut U2, &mut U3, &mut U4, &mut U5) + 'c
    ) -> (DRx<'b, 'c, U1>, DRx<'b, 'c, U2>, DRx<'b, 'c, U3>, DRx<'b, 'c, U4>, DRx<'b, 'c, U5>) where 'c: 'b {
        let (observers, raw) = self.observers_and_get_raw();
        let (a, b, c, d, e) = f(&mut *raw.borrow_mut());
        (
            DRx { value: RefCell::new(a), observers },
            DRx { value: RefCell::new(b), observers },
            DRx { value: RefCell::new(c), observers },
            DRx { value: RefCell::new(d), observers },
            DRx { value: RefCell::new(e), observers }
        )
    }
}

impl<'c, T: 'c> _MRx<'c, 'c, T> for SRx<'c, T> {
    type RawRef<'a> = SRxRefCell<'a, T> where 'c: 'a;

    fn get_raw(&self) -> Self::RawRef<'_> {
        SRxRefCell(&self.value)
    }

    fn observers(&self) -> &RxObservers<'c> {
        &self.observers
    }

    fn observers_and_get_raw(&self) -> (&RxObservers<'c>, Self::RawRef<'_>) {
        (&self.observers, SRxRefCell(&self.value))
    }

    fn trigger(&self) {
        self.observers.trigger()
    }
}

impl<'a, 'c: 'a, T: 'c> _MRx<'a, 'c, T> for DRx<'a, 'c, T> {
    type RawRef<'b> = DRxRefCell<'b, 'a, T> where 'a: 'b;

    fn get_raw(&self) -> Self::RawRef<'_> {
        DRxRefCell(&self.value)
    }

    fn observers(&self) -> &RxObservers<'c> {
        &self.observers
    }

    fn observers_and_get_raw(&self) -> (&RxObservers<'c>, Self::RawRef<'_>) {
        (&self.observers, DRxRefCell(&self.value))
    }

    fn trigger(&self) {
        self.observers.trigger()
    }
}

impl<'c, T: 'c> CRx<'c, T> {
    pub fn new(compute: impl FnMut(&RxContextRef<'c>) -> T + 'c) -> Self {
        CRx(CRxImplImpl::new(compute))
    }
}

impl<'c, T: 'c> SRx<'c, T> {
    pub fn new(value: T) -> Self {
        SRx {
            value: RefCell::new(value),
            observers: RxObservers::new(),
        }
    }

    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }
}

impl<'c, T: 'c, F: FnMut(&RxContextRef<'c>) -> T + 'c> CRxImplImpl<'c, T, F> {
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

impl<'c, T: 'c, F: FnMut(&RxContextRef<'c>) -> T + 'c> CRxImplDyn<'c> for CRxImplImpl<'c, T, F> {
    type T = T;

    fn get<'a>(&'a self, c: &(dyn AsRxContext<'c> + 'c)) -> Ref<'a, Self::T> {
        self.observers.insert(c.as_rx_context());
        self.value.borrow()
    }
}

impl<'c, T: 'c, F: FnMut(&RxContextRef<'c>) -> T + 'c> RxContext for CRxImplImpl<'c, T, F> {
    fn recompute(self: Rc<Self>) {
        self.clone().recompute_without_trigger();
        self.observers.trigger();
    }
}

impl<'c, T: 'c, F: FnMut(&RxContextRef<'c>) -> T + 'c> CRxImplImpl<'c, T, F> {
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
    use super::run_rx::run_rx;
    use super::snapshot_ctx::SNAPSHOT_CTX;

    #[test]
    fn test_srx() {
        let mut rx = SRx::new(vec![1, 2, 3]);
        assert_eq!(rx.get(SNAPSHOT_CTX), &vec![1, 2, 3]);
        rx.set(vec![1, 2, 4]);
        assert_eq!(rx.get(SNAPSHOT_CTX), &vec![1, 2, 4]);
        rx.set(vec![1, 2, 5]);
        assert_eq!(rx.get(SNAPSHOT_CTX), &vec![1, 2, 5]);
    }

    #[test]
    fn test_drx() {
        let mut rx = SRx::new(vec![1, 2, 3]);
        {
            let mut drx = rx.map_mut(|x| x.get_mut(0).unwrap());
            assert_eq!(drx.get(SNAPSHOT_CTX), &1);
            drx.set(2);
            assert_eq!(drx.get(SNAPSHOT_CTX), &2);
        }
        assert_eq!(rx.get(SNAPSHOT_CTX), &vec![2, 2, 5]);
    }

    #[test]
    fn test_drx_split() {
        let mut rx = SRx::new(vec![1, 2, 3]);
        {
            let (mut drx0, mut drx1, mut drx2) = rx.split_map_mut3(|x| {
                let mut iter = x.iter_mut();
                (iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap())
            });
            assert_eq!(drx0.get(SNAPSHOT_CTX), &1);
            assert_eq!(drx1.get(SNAPSHOT_CTX), &2);
            assert_eq!(drx2.get(SNAPSHOT_CTX), &5);
            drx0.set(2);
            drx1.set(3);
            drx2.set(4);
        }
        assert_eq!(rx.get(SNAPSHOT_CTX), &vec![2, 3, 4]);
    }

    #[test]
    fn test_crx() {
        let mut rx = SRx::new(vec![1, 2, 3]);
        let mut crx = CRx::new(|c| rx.get(c)[0] * 2);
        let mut crx2 = CRx::new(|c| *crx.get(c) + rx.get(c)[1] * 10);
        let mut crx3 = crx.map(|x| x.to_string());
        assert_eq!(*crx.get(SNAPSHOT_CTX), 2);
        assert_eq!(*crx2.get(SNAPSHOT_CTX), 22);
        assert_eq!(&*crx3.get(SNAPSHOT_CTX), "2");
        rx.set(vec![2, 3, 4]);
        assert_eq!(*crx.get(SNAPSHOT_CTX), 4);
        assert_eq!(*crx2.get(SNAPSHOT_CTX), 34);
        assert_eq!(&*crx3.get(SNAPSHOT_CTX), "4");
        rx.set(vec![3, 4, 5]);
        assert_eq!(*crx.get(SNAPSHOT_CTX), 6);
        assert_eq!(*crx2.get(SNAPSHOT_CTX), 46);
        assert_eq!(&*crx3.get(SNAPSHOT_CTX), "6");
    }

    #[test]
    fn test_complex_rx_tree() {
        let mut rx1 = SRx::new(vec![1, 2, 3, 4, 5]);
        let (mut rx2_0, mut rx2_1, mut rx2_3, mut rx2_4) = rx1.split_map_mut4(|x| (&mut x[0], &mut x[1], &mut x[3], &mut x[4]));
        let mut rx3 = CRx::new(|c| vec![rx2_0.get(c) * 0, rx2_1.get(c) * 1, rx1.get(c)[2] * 2, rx2_3.get(c) * 3, rx2_4.get(c) * 4]);
        let mut rx4 = CRx::new(|c| rx3.get(c).iter().copied().zip(rx1.get(c).iter().copied()).map(|(a, b)| a + b).collect::<Vec<_>>());
        let (rx5_0, rx5_1, rx5_3) = rx1.split_map3(|x| (&x[0], &x[1], &x[3]));
        assert_eq!(&*rx4.get(SNAPSHOT_CTX), &vec![1, 4, 9, 16, 25]);
        rx2_1.set(8);
        rx2_0.set(25);
        assert_eq!(&*rx4.get(SNAPSHOT_CTX), &vec![25, 16, 9, 16, 25]);
        rx1.set(vec![5, 4, 3, 2, 1]);
        assert_eq!(&*rx4.get(SNAPSHOT_CTX), &vec![5, 8, 9, 8, 5]);
    }

    #[test]
    fn test_run_rx() {
        let mut rx = SRx::new(1);
        let mut rx_snapshots = Vec::new();
        let mut expected_rx_snapshots = Vec::new();
        run_rx(|c| {
            rx_snapshots.push(*rx.get(c))
        });
        for i in 0..1000 {
            rx.set(*rx.get(SNAPSHOT_CTX) + 1);
            expected_rx_snapshots.push(i);
        }
        assert_eq!(rx_snapshots, expected_rx_snapshots);
    }
}
