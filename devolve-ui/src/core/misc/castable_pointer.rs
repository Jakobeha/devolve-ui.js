use std::any::TypeId;
use std::cell::RefCell;
use std::mem;
use std::rc::{Rc, Weak};
use std::sync::Arc;

pub trait Pointer {}

impl <T> Pointer for Box<T> {}

impl <T> Pointer for Rc<T> {}

impl <T> Pointer for Arc<T> {}

impl <T> Pointer for Weak<T> {}

impl <T: Pointer> Pointer for RefCell<T> {}

pub struct CastablePointer<T: Pointer> {
    type_id: TypeId,
    data: T
}

impl <T: Pointer + 'static> From<T> for CastablePointer<T> {
    fn from(data: T) -> Self {
        CastablePointer {
            type_id: TypeId::of::<T>(),
            data
        }
    }
}

impl <T: Pointer> CastablePointer<T> {
    pub fn is<U: 'static>(&self) -> bool {
        self.type_id == TypeId::of::<U>()
    }

    /// This is not technically safe because you must ensure that the struct outside of the pointer is the same
    pub fn downcast<U: 'static>(&self) -> &U {
        if self.is::<U>() {
            unsafe { (&self.data as *const T as *const U).as_ref().unwrap() }
        } else {
            panic!("Type mismatch")
        }
    }

    /// This is not technically safe because you must ensure that the struct outside of the pointer is the same
    pub fn into_downcast<U: 'static>(self) -> U {
        if self.is::<U>() {
            unsafe { mem::transmute(self.data) }
        } else {
            panic!("Type mismatch")
        }
    }
}

impl <T: 'static> CastablePointer<Weak<T>> {
    pub fn upgrade(&self) -> Option<CastablePointer<Rc<T>>> {
        self.data.upgrade().map(|data| CastablePointer::from(data))
    }
}