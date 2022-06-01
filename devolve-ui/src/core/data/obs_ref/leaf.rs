use std::rc::{Rc, Weak};
use crate::core::data::obs_ref::{ObsRefableRoot, ObsRefableChild, ObsRefRootBase, ObsRefChildBase};

// Specific ObsRefable implementations
pub trait Leaf {}

// impl <T> Leaf for T where T: Copy {}
impl Leaf for u8 {}
impl Leaf for u16 {}
impl Leaf for u32 {}
impl Leaf for u64 {}
impl Leaf for u128 {}
impl Leaf for usize {}
impl Leaf for i8 {}
impl Leaf for i16 {}
impl Leaf for i32 {}
impl Leaf for i64 {}
impl Leaf for i128 {}
impl Leaf for isize {}
impl Leaf for f32 {}
impl Leaf for f64 {}
impl Leaf for bool {}
impl Leaf for char {}

impl <T : Leaf> ObsRefableRoot for T {
    type ObsRefImpl = Rc<ObsRefRootBase<T>>;

    fn to_obs_ref(self: Self) -> Self::ObsRefImpl {
        ObsRefRootBase::new(self)
    }
}

impl <Root, T : Leaf> ObsRefableChild<Root> for T {
    type ObsRefImpl = ObsRefChildBase<Root, T>;

    unsafe fn _to_obs_ref_child(this: *mut Self, path: String, root: Weak<ObsRefRootBase<Root>>) -> Self::ObsRefImpl {
        ObsRefChildBase {
            child_value: this,
            path,
            root
        }
    }
}