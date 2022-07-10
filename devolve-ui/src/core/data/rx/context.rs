use std::hash::Hash;
use std::rc::{Rc, Weak};

#[derive(Clone)]
pub enum RxContextRef<'c> {
    Weak(Weak<dyn RxContext + 'c>),
    Strong(Rc<dyn RxContext + 'c>),
}

impl<'c> RxContextRef<'c> {
    pub fn owned(ctx: impl RxContext + 'c) -> Self {
        Self::Strong(Rc::new(ctx))
    }

    pub(super) fn upgrade(&self) -> Option<Rc<dyn RxContext + 'c>> {
        match self {
            RxContextRef::Weak(ref w) => w.upgrade(),
            RxContextRef::Strong(ref s) => Some(s.clone()),
        }
    }

    pub(super) fn as_ptr(&self) -> *const (dyn RxContext + 'c) {
        match self {
            RxContextRef::Weak(x) => x.as_ptr(),
            RxContextRef::Strong(x) => Rc::as_ptr(x)
        }
    }
}

impl<'c> PartialEq<RxContextRef<'c>> for RxContextRef<'c> {
    fn eq(&self, other: &RxContextRef) -> bool {
        self.as_ptr() == other.as_ptr()
    }
}

impl<'c> Eq for RxContextRef<'c> {}

impl<'c> Hash for RxContextRef<'c> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_ptr().hash(state);
    }
}

pub trait AsRxContext<'c> {
    fn as_rx_context(&self) -> RxContextRef<'c>;
}

impl<'c> AsRxContext<'c> for RxContextRef<'c> {
    fn as_rx_context(&self) -> RxContextRef<'c> {
        self.clone()
    }
}

pub trait RxContext {
    fn recompute(self: Rc<Self>);
}
