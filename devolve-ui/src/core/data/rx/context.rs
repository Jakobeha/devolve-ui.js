use std::hash::Hash;
use std::rc::{Rc, Weak};

#[derive(Clone)]
pub enum RxContextRef<'a> {
    Weak(Weak<dyn RxContext + 'a>),
    Strong(Rc<dyn RxContext + 'a>),
}

impl<'a> RxContextRef<'a> {
    pub fn owned(ctx: impl RxContext + 'a) -> Self {
        Self::Strong(Rc::new(ctx))
    }

    pub(super) fn upgrade(&self) -> Option<Rc<dyn RxContext + 'a>> {
        match self {
            RxContextRef::Weak(ref w) => w.upgrade(),
            RxContextRef::Strong(ref s) => Some(s.clone()),
        }
    }

    pub(super) fn as_ptr(&self) -> *const (dyn RxContext + 'a) {
        match self {
            RxContextRef::Weak(x) => x.as_ptr(),
            RxContextRef::Strong(x) => Rc::as_ptr(x)
        }
    }
}

impl<'a> PartialEq<RxContextRef<'a>> for RxContextRef<'a> {
    fn eq(&self, other: &RxContextRef) -> bool {
        self.as_ptr() == other.as_ptr()
    }
}

impl<'a> Eq for RxContextRef<'a> {}

impl<'a> Hash for RxContextRef<'a> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_ptr().hash(state);
    }
}

pub trait AsRxContext<'a> {
    fn as_rx_context(&self) -> RxContextRef<'a>;
}

impl<'a> AsRxContext<'a> for RxContextRef<'a> {
    fn as_rx_context(&self) -> RxContextRef<'a> {
        self.clone()
    }
}

pub trait RxContext {
    fn recompute(self: Rc<Self>);
}
