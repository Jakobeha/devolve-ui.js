use std::cell::{Ref, RefCell, RefMut};
use std::ops::{Deref, DerefMut};
use std::marker::PhantomData;
use crate::core::data::rx::_MRx;

impl<'a, 'c: 'a, T: 'c, R: _MRx<'c, T>> MRxRef<'a, 'c, T, R, <R::RawRef<'a> as MRxRefCell<'a, T>>::RefMut<'a>> {
    pub(super) fn new(rx: &'a R) -> Self {
        MRxRef(rx, rx.get_raw().borrow_mut(), PhantomData)
    }
}

impl<'a, 'c: 'a, T: 'c, R: _MRx<'c, T>> Deref for MRxRef<'a, 'c, T, R, <R::RawRef<'a> as MRxRefCell<'a, T>>::RefMut<'a>> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.1.deref()
    }
}

impl<'a, 'c: 'a, T: 'c, R: _MRx<'c, T>> DerefMut for MRxRef<'a, 'c, T, R, <R::RawRef<'a> as MRxRefCell<'a, T>>::RefMut<'a>> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.1.deref_mut()
    }
}

impl<'a, 'c: 'a, T: 'c, R: DropRef<T>, R2> Drop for MRxRef<'a, 'c, T, R, R2> {
    fn drop(&mut self) {
        self.0.drop_ref();
    }
}

/// Reference to data in an `Rx` which triggers update when it gets dropped.
pub struct MRxRef<'a, 'c: 'a, T: 'c, R: DropRef<T>, R2>(&'a R, R2, PhantomData<&'c T>);

pub struct DRxRef<'b, 'a, T>(pub(super) Ref<'b, &'a mut T>);

pub struct DRxRefMut<'b, 'a, T>(pub(super) RefMut<'b, &'a mut T>);

pub struct SRxRefCell<'a, T>(pub(super) &'a RefCell<T>);
pub struct DRxRefCell<'b, 'a, T>(pub(super) &'b RefCell<&'a mut T>);

pub trait MRxRefCell<'a, T> {
    type Ref<'b>: Deref<Target = T> where Self: 'b, 'a: 'b;
    type RefMut<'b>: DerefMut<Target = T> where Self: 'b, 'a: 'b;

    fn borrow(&self) -> Self::Ref<'_>;
    fn borrow_mut(&self) -> Self::RefMut<'_>;
    fn replace(&self, new_value: T);
}

pub trait DropRef<T> {
    fn drop_ref(&self);
}

impl<'a, T> MRxRefCell<'a, T> for SRxRefCell<'a, T> {
    type Ref<'b> = Ref<'b, T> where 'a: 'b;
    type RefMut<'b> = RefMut<'b, T> where 'a: 'b;

    fn borrow(&self) -> Self::Ref<'_> {
        self.0.borrow()
    }

    fn borrow_mut(&self) -> Self::RefMut<'_> {
        self.0.borrow_mut()
    }

    fn replace(&self, new_value: T) {
        self.0.replace(new_value);
    }
}

impl<'b, 'a, T> MRxRefCell<'b, T> for DRxRefCell<'b, 'a, T> {
    type Ref<'d> = DRxRef<'d, 'a, T> where 'b: 'd;
    type RefMut<'d> = DRxRefMut<'d, 'a, T> where 'b: 'd;

    fn borrow(&self) -> Self::Ref<'_> {
        DRxRef(self.0.borrow())
    }

    fn borrow_mut(&self) -> Self::RefMut<'_> {
        DRxRefMut(self.0.borrow_mut())
    }

    fn replace(&self, new_value: T) {
        **self.0.borrow_mut() = new_value;
    }
}

impl<'c, T: 'c, R: _MRx<'c, T>> DropRef<T> for R {
    fn drop_ref(&self) {
        self.trigger();
    }
}

impl<'b, 'a, T> Deref for DRxRef<'b, 'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<'b, 'a, T> Deref for DRxRefMut<'b, 'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<'b, 'a, T> DerefMut for DRxRefMut<'b, 'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}
