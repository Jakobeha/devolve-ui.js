//! `Rx` means "reactive value" (or "reactive X"). It is a wrapper for a value which changes,
//! and these changes trigger dependencies to re-run and change themselves. You can't access the
//! value directly, instead you use an associated function like `run_rx` to access it in a closure
//! which can re-run whenever the dependency changes. You can create new `Rx`s from old ones.

pub mod context;
pub mod observers;
pub mod run_rx;
pub mod snapshot_ctx;
pub mod refs;

use std::cell::{Cell, Ref, RefCell};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use refs::MapRef;
use crate::core::data::rx::refs::{DRxRef, DRxRefCell, MRxRef, MRxRefCell, SRxRefCell};
use crate::core::data::rx::context::{AsRxContext, RxContext, RxContextRef};
use crate::core::data::rx::observers::RxObservers;
use crate::core::data::rx::run_rx::RunRxContext;
use crate::core::data::rx::context::assert_is_ctx_variant;
use crate::core::misc::assert_variance::assert_is_covariant;
use crate::core::misc::map_split_n::MapSplitN;

pub trait ARx<'ctx, T: 'ctx> {
    fn get(&self) -> &T;

    fn map<'a, U: 'ctx>(&'a self, f: impl FnMut(&T) -> &U + 'a) -> DCRx<'a, 'ctx, U> where Self: Sized, 'ctx: 'a;
    fn split_map2<'a, U1: 'ctx, U2: 'ctx>(
        &'a self,
        f: impl FnMut(&T) -> (U1, U2) + 'a
    ) -> (CRx<'ctx, U1>, CRx<'ctx, U2>) where Self: Sized, 'ctx: 'a;
}

pub trait Var<'ctx, T: 'ctx> {
    fn get(&self) -> &T;
    fn set(&self, new_value: T);

    fn map<'a, U>(&'a mut self, f: impl FnMut(&'a mut T) -> &'a mut U + 'a) -> DVar<'a, 'ctx, U> where Self: Sized, 'ctx: 'a;
    fn split_map2<'a, U1, U2>(
        &'a mut self,
        f: impl FnMut(&'a mut T) -> (&'a mut U1, &'a mut U2) + 'a
    ) -> (DVar<'a, 'ctx, U1>, DVar<'a, 'ctx, U2>) where Self: Sized, 'ctx: 'a;
}

/// Source `Var`. Serves as an input to `Rx`s which can be accessed and mutated directly.
/// Contains its value.
pub struct SVar<'ctx, T: 'ctx> {
    current: T,
    next: Cell<Option<T>>,
    context: &'ctx RxContext<'ctx>
}
assert_is_ctx_variant!(for[T] (SRx<'ctx, T>) over 'ctx);

/// Derived `Var`. Serves as an input to `Rx`s which can be accessed and mutated directly.
/// Contains part of a value from a `SVar`.
pub struct DVar<'a, 'ctx, T> {
    current: &'a mut T,
    next: Cell<Option<&'a mut T>>,
    context: &'ctx RxContext<'ctx>
}
assert_is_ctx_variant!(for['a, T] (DVar<'a, 'ctx, T>) over 'ctx);

/// Source `Rx`. A computed value from `Var`s and other `Rx`s. Contains its value.
pub struct SRx<'ctx, T: 'ctx> {
    current: T,
    next: Cell<Option<T>>,
    context: &'ctx RxContext<'ctx>
}
assert_is_ctx_variant!(for[T] (CRx<'ctx, T>) over 'ctx);
assert_is_covariant!(for['ctx], (CRx<'ctx, T>) over T)

/// Derived `Rx`. A computed value from `Var`s and other `Rx`s. Contains a part of a value from an `SRx`.
pub struct DRx<'a, 'ctx, T> {
    current: &'a T,
    next: Cell<Option<&'a T>>,
    context: &'ctx RxContext<'ctx>
}
assert_is_ctx_variant!(for['a, T] (DCRx<'ctx, T>) over 'ctx);

// trait CRxImplDyn<'ctx cov, T cov>: RxContext
trait CRxImplDyn: RxContext {
    // fn get<'a, 'ctx: 'ctx + 'a>(&'a self, c: &(dyn AsRxContext<'ctx2> + '_)) -> Ref<'a, T>;
    fn get<'a, 'ctx2: 'a>(&'a self, c: &(dyn AsRxContext<'ctx2> + '_)) -> Ref<'a, Self::T>;
}

struct CRxImplImpl<'ctx, T: 'ctx, F: FnMut(&RxContextRef<'ctx>) -> T + 'ctx> {
    value: T,
    compute: RefCell<F>,
    observers: RxObservers<'ctx>,
}
// assert_is_ctx_variant!(for[T, F] (CRxImplImpl<'ctx, T, F>) over 'ctx where {'__a, '__b} [F: FnMut(&RxContextRef<'ctx>) -> T] [F: FnMut(&RxContextRef<'__a>) -> T] [F: FnMut(&RxContextRef<'__b>) -> T]);

/*pub(super) */pub trait _MRx<'ctx, T: 'ctx>: Rx<'ctx, T> {
    type RawRef<'b>: MRxRefCell<'b, T> where Self: 'b;

    fn get_raw(&self) -> Self::RawRef<'_>;
    fn get_raw_mut(&mut self) -> &mut T;
    fn observers(&self) -> &RxObservers<'ctx>;
    fn observers_and_get_raw_mut(&mut self) -> (&RxObservers<'ctx>, &mut T);

    fn trigger(&self);
}

impl<'ctx, T: 'ctx> Rx<'ctx, T> for CRx<'ctx, T> {
    type Ref<'a> = Ref<'a, T> where Self: 'a, 'ctx: 'a;

    fn get<'a, 'ctx2: 'ctx + 'a>(&'a self, c: &(dyn AsRxContext<'ctx2> + '_)) -> Self::Ref<'a> {
        self.0.get(c)
    }
}

impl<'ctx, T: 'ctx> Rx<'ctx, T> for SRx<'ctx, T> {
    type Ref<'a> = Ref<'a, T> where Self: 'a, 'ctx: 'a;

    fn get<'a, 'ctx2: 'ctx + 'a>(&'a self, c: &(dyn AsRxContext<'ctx2> + '_)) -> Self::Ref<'a> {
        self.observers.insert(c.as_rx_context());
        self.value.borrow()
    }
}

impl<'a, 'ctx: 'a, T: 'ctx> Rx<'ctx, T> for DVar<'a, 'ctx, T> {
    type Ref<'b> = DRxRef<'b, 'a, T> where Self: 'b, 'ctx: 'b;

    fn get<'b, 'ctx2: 'ctx + 'b>(&'b self, c: &(dyn AsRxContext<'ctx2> + '_)) -> Self::Ref<'b> {
        self.observers.insert(c.as_rx_context());
        DRxRef(self.value.borrow())
    }
}

impl<'ctx, T: 'ctx, R: _MRx<'ctx, T>> Var<'ctx, T> for R {
    type RefMut<'a> = MRxRef<'a, 'ctx, T, Self, <R::RawRef<'a> as MRxRefCell<'a, T>>::RefMut> where Self: 'a, 'ctx: 'a;

    fn get_mut<'a>(&'a mut self, c: &(dyn AsRxContext<'ctx> + 'ctx)) -> Self::RefMut<'a> where 'ctx: 'a {
        self.observers().insert(c.as_rx_context());
        MRxRef::new(self)
    }

    fn set(&self, new_value: T) {
        self.get_raw().replace(new_value);
        self.trigger();
    }

    fn modify(&'ctx self, f: impl Fn(&mut T) + 'ctx) {
        f(&mut *self.get_raw().borrow_mut());
        self.trigger();
        let observers = self.observers();
        // Need to add observer after the trigger so that we don't re-trigger and recurse
        observers.insert(RxContextRef::owned(RunRxContext::<'ctx, _>::new(move |c| {
            // equivalent to `f(self.get_mut(c).deref_mut())`, except we don't trigger
            observers.insert(c.as_rx_context());
            f(&mut self.get_raw().borrow_mut());
        })));
    }

    fn map_mut<'a, U>(&'a mut self, f: impl Fn(&'a mut T) -> &'a mut U + 'ctx) -> DVar<'a, 'ctx, U> where 'ctx: 'a {
        let (observers, raw_mut) = self.observers_and_get_raw_mut();
        DVar {
            value: RefCell::new(f(raw_mut)),
            observers
        }
    }

    fn split_map_mut2<'a, U1, U2>(
        &'a mut self,
        f: impl Fn(&'a mut T) -> (&'a mut U1, &'a mut U2) + 'ctx
    ) -> (DVar<'a, 'ctx, U1>, DVar<'a, 'ctx, U2>) where 'ctx: 'a {
        let (observers, raw_mut) = self.observers_and_get_raw_mut();
        let (a, b) = f(raw_mut);
        (
            DVar { value: RefCell::new(a), observers },
            DVar { value: RefCell::new(b), observers }
        )
    }

    fn split_map_mut3<'a, U1, U2, U3>(
        &'a mut self,
        f: impl Fn(&'a mut T) -> (&'a mut U1, &'a mut U2, &'a mut U3) + 'ctx
    ) -> (DVar<'a, 'ctx, U1>, DVar<'a, 'ctx, U2>, DVar<'a, 'ctx, U3>) where 'ctx: 'a {
        let (observers, raw_mut) = self.observers_and_get_raw_mut();
        let (a, b, c) = f(raw_mut);
        (
            DVar { value: RefCell::new(a), observers },
            DVar { value: RefCell::new(b), observers },
            DVar { value: RefCell::new(c), observers }
        )
    }

    fn split_map_mut4<'a, U1, U2, U3, U4>(
        &'a mut self,
        f: impl Fn(&'a mut T) -> (&'a mut U1, &'a mut U2, &'a mut U3, &'a mut U4) + 'ctx
    ) -> (DVar<'a, 'ctx, U1>, DVar<'a, 'ctx, U2>, DVar<'a, 'ctx, U3>, DVar<'a, 'ctx, U4>) where 'ctx: 'a {
        let (observers, raw_mut) = self.observers_and_get_raw_mut();
        let (a, b, c, d) = f(raw_mut);
        (
            DVar { value: RefCell::new(a), observers },
            DVar { value: RefCell::new(b), observers },
            DVar { value: RefCell::new(c), observers },
            DVar { value: RefCell::new(d), observers }
        )
    }

    fn split_map_mut5<'a, U1, U2, U3, U4, U5>(
        &'a mut self,
        f: impl Fn(&'a mut T) -> (&'a mut U1, &'a mut U2, &'a mut U3, &'a mut U4, &'a mut U5) + 'ctx
    ) -> (DVar<'a, 'ctx, U1>, DVar<'a, 'ctx, U2>, DVar<'a, 'ctx, U3>, DVar<'a, 'ctx, U4>, DVar<'a, 'ctx, U5>) where 'ctx: 'a {
        let (observers, raw_mut) = self.observers_and_get_raw_mut();
        let (a, b, c, d, e) = f(raw_mut);
        (
            DVar { value: RefCell::new(a), observers },
            DVar { value: RefCell::new(b), observers },
            DVar { value: RefCell::new(c), observers },
            DVar { value: RefCell::new(d), observers },
            DVar { value: RefCell::new(e), observers }
        )
    }
}

impl<'ctx, T: 'ctx> _MRx<'ctx, T> for SRx<'ctx, T> {
    type RawRef<'a> = SRxRefCell<'a, T> where 'ctx: 'a;

    fn get_raw(&self) -> Self::RawRef<'_> {
        SRxRefCell(&self.value)
    }

    fn get_raw_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }

    fn observers(&self) -> &RxObservers<'ctx> {
        &self.observers
    }

    fn observers_and_get_raw_mut(&mut self) -> (&RxObservers<'ctx>, &mut T) {
        (&self.observers, self.value.get_mut())
    }

    fn trigger(&self) {
        self.observers.trigger()
    }
}

impl<'a, 'ctx: 'a, T: 'ctx> _MRx<'ctx, T> for DVar<'a, 'ctx, T> {
    type RawRef<'b> = DRxRefCell<'b, 'a, T> where 'a: 'b;

    fn get_raw(&self) -> Self::RawRef<'_> {
        DRxRefCell(&self.value)
    }

    fn get_raw_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }

    fn observers(&self) -> &RxObservers<'ctx> {
        &self.observers
    }

    fn observers_and_get_raw_mut(&mut self) -> (&RxObservers<'ctx>, &mut T) {
        (&self.observers, self.value.get_mut())
    }

    fn trigger(&self) {
        self.observers.trigger()
    }
}

impl<'ctx, T: 'ctx> CRx<'ctx, T> {
    pub fn new(compute: impl FnMut(&RxContextRef<'ctx>) -> T + 'ctx) -> Self {
        CRx(CRxImplImpl::new(compute))
    }
}

impl<'ctx, T: 'ctx> SRx<'ctx, T> {
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

impl<'ctx, T: 'ctx, F: FnMut(&RxContextRef<'ctx>) -> T + 'ctx> CRxImplImpl<'ctx, T, F> {
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

impl<'ctx, T: 'ctx, F: FnMut(&RxContextRef<'ctx>) -> T + 'ctx> CRxImplDyn for CRxImplImpl<'ctx, T, F> {
    type T = T;

    // fn get<'a, 'ctx2: 'ctx + 'a>(&'a self, c: &(dyn AsRxContext<'ctx2> + '_)) -> Ref<'a, Self::T>
    fn get<'a, 'ctx2: 'a>(&'a self, c: &(dyn AsRxContext<'ctx2> + '_)) -> Ref<'a, Self::T> {
        // Here we must extend lifetime because we can't enforce 'ctx2: 'ctx and keep variance
        self.observers.insert(unsafe { std::mem::transmute::<RxContextRef<'ctx2>, RxContextRef<'ctx>>(c.as_rx_context()) });
        self.value.borrow()
    }
}

impl<'ctx, T: 'ctx, F: FnMut(&RxContextRef<'ctx>) -> T + 'ctx> RxContext for CRxImplImpl<'ctx, T, F> {
    fn recompute(self: Rc<Self>) {
        self.clone().recompute_without_trigger();
        self.observers.trigger();
    }
}

impl<'ctx, T: 'ctx, F: FnMut(&RxContextRef<'ctx>) -> T + 'ctx> CRxImplImpl<'ctx, T, F> {
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
        let rx = SRx::new(vec![1, 2, 3]);
        assert_eq!(rx.get(SNAPSHOT_CTX).deref(), &vec![1, 2, 3]);
        rx.set(vec![1, 2, 4]);
        assert_eq!(rx.get(SNAPSHOT_CTX).deref(), &vec![1, 2, 4]);
        rx.set(vec![1, 2, 5]);
        assert_eq!(rx.get(SNAPSHOT_CTX).deref(), &vec![1, 2, 5]);
    }

    #[test]
    fn test_drx() {
        let mut rx = SRx::new(vec![1, 2, 3]);
        {
            let drx = rx.map_mut(|x| x.get_mut(0).unwrap());
            assert_eq!(drx.get(SNAPSHOT_CTX).deref(), &1);
            drx.set(2);
            assert_eq!(drx.get(SNAPSHOT_CTX).deref(), &2);
        }
        {
            let drx2 = rx.map_mut(|x| x.get_mut(2).unwrap());
            assert_eq!(drx2.get(SNAPSHOT_CTX).deref(), &3);
            drx2.modify(|x| *x += 2);
            assert_eq!(drx2.get(SNAPSHOT_CTX).deref(), &5);
        }
        assert_eq!(rx.get(SNAPSHOT_CTX).deref(), &vec![2, 2, 5]);
    }

    #[test]
    fn test_drx_split() {
        let mut rx = SRx::new(vec![1, 2, 3]);
        {
            let (drx0, drx1, drx2) = rx.split_map_mut3(|x| {
                let mut iter = x.iter_mut();
                (iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap())
            });
            assert_eq!(drx0.get(SNAPSHOT_CTX).deref(), &1);
            assert_eq!(drx1.get(SNAPSHOT_CTX).deref(), &2);
            assert_eq!(drx2.get(SNAPSHOT_CTX).deref(), &3);
            drx0.set(2);
            drx1.set(3);
            drx2.set(4);
        }
        assert_eq!(rx.get(SNAPSHOT_CTX).deref(), &vec![2, 3, 4]);
    }

    #[test]
    fn test_crx() {
        let rx = SRx::new(vec![1, 2, 3]);
        {
            let crx = CRx::new(|c| rx.get(c)[0] * 2);
            let crx2 = CRx::new(|c| *crx.get(c) + rx.get(c)[1] * 10);
            let crx3 = crx.map(|x| x.to_string());
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
    }

    #[test]
    fn test_complex_rx_tree() {
        let mut rx1 = SRx::new(vec![1, 2, 3, 4]);
        {
            let (rx2_0, rx2_1, rx2_2, rx2_3) = rx1.split_map_mut4(|x| {
                let mut iter = x.iter_mut();
                (iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap())
            });
            let rx1_alt = CRx::new(|c| vec![*rx2_0.get(c), *rx2_1.get(c), *rx2_2.get(c), *rx2_3.get(c)]);
            let rx3 = CRx::new(|c| vec![*rx2_0.get(c) * 0, *rx2_1.get(c) * 1, *rx2_2.get(c) * 3, *rx2_3.get(c) * 4]);
            let rx4 = CRx::new(|c| rx3.get(c).iter().copied().zip(rx1_alt.get(c).iter().copied()).map(|(a, b)| a + b).collect::<Vec<_>>());
            let (_rx5_0, _rx5_1, _rx5_3) = rx4.split_map_ref3(|x| (&x[0], &x[1], &x[3]));
            assert_eq!(&*rx4.get(SNAPSHOT_CTX), &vec![1, 4, 9, 16, 25]);
            rx2_1.set(8);
            rx2_0.set(25);
            assert_eq!(&*rx4.get(SNAPSHOT_CTX), &vec![25, 16, 9, 16, 25]);
        }
        rx1.set(vec![5, 4, 3, 2, 1]);
        {
            let (rx2_0, rx2_1, rx2_2, rx2_3) = rx1.split_map_mut4(|x| {
                let mut iter = x.iter_mut();
                (iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap())
            });
            let rx1_alt = CRx::new(|c| vec![*rx2_0.get(c), *rx2_1.get(c), *rx2_2.get(c), *rx2_3.get(c)]);
            let rx3 = CRx::new(|c| vec![*rx2_0.get(c) * 0, *rx2_1.get(c) * 1, *rx2_2.get(c) * 3, *rx2_3.get(c) * 4]);
            let rx4 = CRx::new(|c| rx3.get(c).iter().copied().zip(rx1_alt.get(c).iter().copied()).map(|(a, b)| a + b).collect::<Vec<_>>());
            let (_rx5_0, _rx5_1, _rx5_3) = rx4.split_map_ref3(|x| (&x[0], &x[1], &x[3]));
            assert_eq!(&*rx4.get(SNAPSHOT_CTX), &vec![5, 8, 9, 8, 5]);
        }
    }

    #[test]
    fn test_run_rx() {
        let rx = SRx::new(1);
        let mut rx_snapshots = Vec::new();
        let mut expected_rx_snapshots = Vec::new();
        run_rx(|c| {
            rx_snapshots.push(*rx.get(c))
        });
        for i in 0..1000 {
            let new_value = *rx.get(SNAPSHOT_CTX) + 1;
            rx.set(new_value);
            expected_rx_snapshots.push(i + 1);
        }
        expected_rx_snapshots.push(1001);
        assert_eq!(rx_snapshots, expected_rx_snapshots);
    }
}
