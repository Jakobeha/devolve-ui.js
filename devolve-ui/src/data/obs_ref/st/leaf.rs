use std::rc::{Rc, Weak};
use crate::data::obs_ref::st::{ObsRefableRoot, ObsRefableChild, ObsRefRootBase, ObsRefChildBase, SubCtx, ObsRefPending};

// Specific ObsRefable implementations
pub trait Leaf {}

pub macro derive_obs_ref_leaf($name:ident $( < $( $param:ident ),* > )? $( where ( $($where_tt:tt)* ) )? ) {
    impl $( < $( $param ),* > )? Leaf for $name $( < $( $param ),* > )? {}

    impl <$( $( $param ),* , )? S: SubCtx> ObsRefableRoot<S> for $name $( < $( $param ),* > )?  $( where $($where_tt)* )? {
        type ObsRefImpl = Rc<ObsRefRootBase<$name $( < $( $param ),* > )?, S>>;

        fn into_obs_ref(self) -> Self::ObsRefImpl {
            ObsRefRootBase::new(self)
        }
    }

    impl <Root, $( $( $param ),* , )? S: SubCtx> ObsRefableChild<Root, S> for $name $( < $( $param ),* > )? $( where $($where_tt)* )? {
        type ObsRefImpl = ObsRefChildBase<Root, $name $( < $( $param ),* > )?, S>;

        unsafe fn _as_obs_ref_child(this: *mut Self, ancestors_pending: &[Weak<ObsRefPending<S>>], parent_pending: &Rc<ObsRefPending<S>>, path: String, root: Rc<ObsRefRootBase<Root, S>>) -> Self::ObsRefImpl {
            ObsRefChildBase::new(this, ancestors_pending, parent_pending, path, root)
        }
    }
}

// impl <T: Copy> Leaf for T {}
// impl <T: Zst> !Leaf for T {}
derive_obs_ref_leaf!(u8);
derive_obs_ref_leaf!(u16);
derive_obs_ref_leaf!(u32);
derive_obs_ref_leaf!(u64);
derive_obs_ref_leaf!(u128);
derive_obs_ref_leaf!(usize);
derive_obs_ref_leaf!(i8);
derive_obs_ref_leaf!(i16);
derive_obs_ref_leaf!(i32);
derive_obs_ref_leaf!(i64);
derive_obs_ref_leaf!(i128);
derive_obs_ref_leaf!(isize);
derive_obs_ref_leaf!(f32);
derive_obs_ref_leaf!(f64);
derive_obs_ref_leaf!(bool);
derive_obs_ref_leaf!(char);