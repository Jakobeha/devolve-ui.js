use std::marker::{PhantomData, PhantomPinned};
use std::rc::{Rc, Weak};
use crate::core::data::obs_ref::st::{ObsRefableRoot, ObsRefableChild, ObsRefRootBase, SubCtx, ObsRefPending, ObsRef, ObsDeref, Observer};

pub trait Zst {
    const INSTANCE: Self;
}

// impl <T> Zst for T where T is a ZST

/// Special `ObsRef` implementation for ZSTs:
/// they will never mutate, so we can return a dummy ZST obs-ref.
struct ZstObsRef<Root, T: Zst, S: SubCtx>(PhantomData<(Root, T, S)>);

impl <Root, T: Zst, S: SubCtx> ObsRef<Root, T, S> for ZstObsRef<Root, T, S> {
    fn i(&self, _s: S::Input<'_>) -> &T {
        &T::INSTANCE
    }

    fn m(&mut self, _s: S::Input<'_>) -> ObsDeref<Root, T, S> {
        ObsDeref::zst(&T::INSTANCE)
    }

    fn after_mutate(&self, _observer: Observer<Root, S>) {

    }

    fn base(&self) -> &Rc<ObsRefRootBase<Root, S>> {
        panic!("ZstObsRefRoot::base() not implemented")
    }
}

pub macro derive_obs_refable_zst($name:tt $( < $( $param:ident ),* > )?) {
    impl $( < $( $param ),* > )? Zst for $name $( < $( $param ),* > )? {
        const INSTANCE: Self = Self;
    }

    /// Special `ObsRefableRoot` implementation for ZSTs:
    /// they will never mutate, so we can return a dummy ZST obs-ref.
    impl <$( $( $param ),* , )? S: SubCtx> ObsRefableRoot<S> for $name $( < $( $param ),* > )? {
        type ObsRefImpl = ZstObsRef<$name $( < $( $param ),* > )?, $name $( < $( $param ),* > )?, S>;

        fn into_obs_ref(self) -> Self::ObsRefImpl {
            ZstObsRef(PhantomData)
        }
    }

    /// Special `ObsRefableRoot` implementation for ZSTs:
    /// they will never mutate, so we can return a dummy ZST obs-ref.
    impl <Root, $( $( $param ),* , )? S: SubCtx> ObsRefableChild<Root, S> for $name $( < $( $param ),* > )? {
        type ObsRefImpl = ZstObsRef<Root, $name $( < $( $param ),* > )?, S>;

        unsafe fn _as_obs_ref_child(
            _this: *mut Self,
            _ancestors_pending: &[Weak<ObsRefPending<S>>],
            _parent_pending: &Rc<ObsRefPending<S>>,
            _path: String,
            _root: Rc<ObsRefRootBase<Root, S>>
        ) -> Self::ObsRefImpl {
            ZstObsRef(PhantomData)
        }
    }
}

derive_obs_refable_zst!(());
derive_obs_refable_zst!(PhantomData<T>);
derive_obs_refable_zst!(PhantomPinned);