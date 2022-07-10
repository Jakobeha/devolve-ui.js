use std::rc::{Rc, Weak};
use crate::core::data::rx::context::{AsRxContext, RxContext, RxContextRef};

pub struct AsSnapshotCtx;
enum NeverCtx {}

/// Context which returns the `Rx` at its current state.
/// Be careful using this, as when the `Rx` changes, any getter using `SNAPSHOT_CTX` won't recompute.
pub const SNAPSHOT_CTX: &'static AsSnapshotCtx = &AsSnapshotCtx;

impl<'c> AsRxContext<'c> for AsSnapshotCtx {
    fn as_rx_context(&self) -> RxContextRef<'c> {
        RxContextRef::Weak(Weak::<NeverCtx>::new())
    }
}

impl RxContext for NeverCtx {
    fn recompute(self: Rc<Self>) {
        match *self.as_ref() {}
    }
}