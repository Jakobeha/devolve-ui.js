use std::cell::{Cell, Ref};
use std::ops::Deref;
use std::ptr::NonNull;
use transmute::transmute;

pub trait MapSplitN<'a, T: ?Sized + 'a> {
    type Self_<U: ?Sized + 'a>: MapSplitN<'a, U>;

    fn map_split3<U1, U2, U3>(self, f: impl FnOnce(&T) -> (&U1, &U2, &U3)) -> (Self::Self_<U1>, Self::Self_<U2>, Self::Self_<U3>);
    fn map_split4<U1, U2, U3, U4>(self, f: impl FnOnce(&T) -> (&U1, &U2, &U3, &U4)) -> (Self::Self_<U1>, Self::Self_<U2>, Self::Self_<U3>, Self::Self_<U4>);
    fn map_split5<U1, U2, U3, U4, U5>(self, f: impl FnOnce(&T) -> (&U1, &U2, &U3, &U4, &U5)) -> (Self::Self_<U1>, Self::Self_<U2>, Self::Self_<U3>, Self::Self_<U4>, Self::Self_<U5>);
}

pub struct _Ref<'b, T: ?Sized + 'b> {
    // NB: we use a pointer instead of `&'b T` to avoid `noalias` violations, because a
    // `Ref` argument doesn't hold immutability for its whole scope, only until it drops.
    // `NonNull` is also covariant over `T`, just like we would have with `&T`.
    value: NonNull<T>,
    borrow: _BorrowRef<'b>,
}

impl<'b, T: ?Sized + 'b> Deref for _Ref<'b, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.value.as_ref() }
    }
}

#[allow(dead_code)]
#[derive(Clone)]
struct _BorrowRef<'b> {
    borrow: &'b Cell<_BorrowFlag>,
}

type _BorrowFlag = isize;

impl<'a, T: ?Sized + 'a> MapSplitN<'a, T> for Ref<'a, T> {
    type Self_<U: ?Sized + 'a> = Ref<'a, U>;

    fn map_split3<U1, U2, U3>(self, f: impl FnOnce(&T) -> (&U1, &U2, &U3)) -> (Self::Self_<U1>, Self::Self_<U2>, Self::Self_<U3>) {
        let orig = unsafe { transmute::<Ref<'a, T>, _Ref<'a, T>>(self) };

        let (a, b, c) = f(&*orig);
        let borrow1 = orig.borrow.clone();
        let borrow2 = orig.borrow.clone();

        let result = (
            _Ref { value: NonNull::from(a), borrow: borrow1 },
            _Ref { value: NonNull::from(b), borrow: borrow2 },
            _Ref { value: NonNull::from(c), borrow: orig.borrow },
        );

        unsafe { transmute(result) }
    }

    fn map_split4<U1, U2, U3, U4>(self, f: impl FnOnce(&T) -> (&U1, &U2, &U3, &U4)) -> (Self::Self_<U1>, Self::Self_<U2>, Self::Self_<U3>, Self::Self_<U4>) {
        let orig = unsafe { transmute::<Ref<'a, T>, _Ref<'a, T>>(self) };

        let (a, b, c, d) = f(&*orig);
        let borrow1 = orig.borrow.clone();
        let borrow2 = orig.borrow.clone();
        let borrow3 = orig.borrow.clone();

        let result = (
            _Ref { value: NonNull::from(a), borrow: borrow1 },
            _Ref { value: NonNull::from(b), borrow: borrow2 },
            _Ref { value: NonNull::from(c), borrow: borrow3 },
            _Ref { value: NonNull::from(d), borrow: orig.borrow }
        );

        unsafe { transmute(result) }
    }

    fn map_split5<U1, U2, U3, U4, U5>(self, f: impl FnOnce(&T) -> (&U1, &U2, &U3, &U4, &U5)) -> (Self::Self_<U1>, Self::Self_<U2>, Self::Self_<U3>, Self::Self_<U4>, Self::Self_<U5>) {
        let orig = unsafe { transmute::<Ref<'a, T>, _Ref<'a, T>>(self) };

        let (a, b, c, d, e) = f(&*orig);
        let borrow1 = orig.borrow.clone();
        let borrow2 = orig.borrow.clone();
        let borrow3 = orig.borrow.clone();
        let borrow4 = orig.borrow.clone();

        let result = (
            _Ref { value: NonNull::from(a), borrow: borrow1 },
            _Ref { value: NonNull::from(b), borrow: borrow2 },
            _Ref { value: NonNull::from(c), borrow: borrow3 },
            _Ref { value: NonNull::from(d), borrow: borrow4 },
            _Ref { value: NonNull::from(e), borrow: orig.borrow }
        );

        unsafe { transmute(result) }
    }
}