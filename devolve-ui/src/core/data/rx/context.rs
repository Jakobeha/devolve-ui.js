use std::hash::Hash;
use std::rc::{Rc, Weak};
use derive_more::{Deref, DerefMut, From};

#[derive(Clone, From)]
pub enum RxContextRef {
    Weak(Weak<dyn RxContext>),
    Strong(Rc<dyn RxContext>)
}

impl RxContextRef {
    pub fn owned(ctx: impl RxContext) -> Self {
        Self::Strong(Rc::new(ctx))
    }

    pub(super) fn upgrade(&self) -> Option<Rc<dyn RxContext>> {
        match self {
            RxContextRef::Weak(ref w) => w.upgrade(),
            RxContextRef::Strong(ref s) => s.clone(),
        }
    }

    fn as_ptr(&self) -> *const dyn RxContext {
        match self {
            RxContextRef::Weak(x) => x.as_ptr(),
            RxContextRef::Strong(x) => x as *const dyn RxContext
        }
    }
}

impl PartialEq<RxContextRef> for RxContextRef {
    fn eq(&self, other: &RxContextRef) -> bool {
        self.as_ptr() == other.as_ptr()
    }
}

impl Eq for RxContextRef {}

impl Hash for RxContextRef {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_ptr().hash(state);
    }
}

pub trait AsRxContext {
    fn as_rx_context(&self) -> RxContextRef;
}

impl AsRxContext for RxContextRef {
    fn as_rx_context(&self) -> RxContextRef {
        self.clone()
    }
}

pub trait RxContext {
    fn recompute(&self);
}
