//! Component state which is implicitly passed to and can be accessed from children.
//!
//! See `devolve_ui::core::hooks::state` type for more information about state.
//! This is similar except its created via `use_provide` and accessible in children `use_consume`.
//! For each type of context, you must create a static const `context` object which functions as a
//! shared key for both `use_provide` and `use_consume` and also contains the underlying type.
//! If a child calls `use_provide` with the same context as its parent, then the child's context
//! will shadow in its own children.

use std::any::{Any, TypeId};
use std::marker::{PhantomData, PhantomPinned};
use crate::core::component::context::{VComponentContext, VContext};
use crate::core::component::update_details::{UpdateBacktrace, UpdateDetails};
use crate::core::view::view::VViewData;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[derive(Debug)]
pub struct ContextIdSource<T: Any>(PhantomData<T>, PhantomPinned);

impl <T: Any> ContextIdSource<T> {
    pub const fn new() -> Self {
        ContextIdSource(PhantomData, PhantomPinned)
    }
}

pub type ContextId<T> = *const ContextIdSource<T>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AnonContextId(usize);

impl <T: Any> From<*const ContextIdSource<T>> for AnonContextId {
    fn from(id: *const ContextIdSource<T>) -> Self {
        AnonContextId(id as usize)
    }
}

#[derive(Debug)]
pub struct ContextState<T: Any, ViewData: VViewData> {
    id: ContextId<T>,
    phantom: PhantomData<ViewData>
}

pub fn use_provide<'a, 'a0: 'a, T: Any, ViewData: VViewData + 'a>(
    c: &mut impl VComponentContext<'a, 'a0, ViewData=ViewData>,
    id: ContextId<T>,
    get_initial: impl FnOnce() -> Box<T>
) -> ContextState<T, ViewData> {
    let local_contexts = c.local_contexts();
    if !local_contexts.contains_key(&id.into()) {
        local_contexts.insert(id.into(), get_initial());
    }
    ContextState {
        id,
        phantom: PhantomData
    }
}

pub fn use_consume<'a, 'a0: 'a, T: Any, ViewData: VViewData + 'a>(
    c: &mut impl VComponentContext<'a, 'a0, ViewData=ViewData>,
    id: ContextId<T>
) -> ContextState<T, ViewData> {
    let existing = c.get_mut_context(&id.into());
    assert!(existing.is_some(), "context with id ({:?}) not found in parent", id);
    ContextState {
        id,
        phantom: PhantomData
    }
}

impl <T: Any, ViewData: VViewData> ContextState<T, ViewData> {
    pub fn get<'a: 'b, 'b>(&self, c: &'b impl VContext<'a, ViewData=ViewData>) -> &'b T where ViewData: 'b {
        let value_any = c
            .get_context(&self.id.into())
            .unwrap_or_else(|| panic!("context with id ({:?}) not found in parent", self.id));
        assert!(value_any.is::<T>(), "context with id ({:?}) has wrong type: expected {:?}, got {:?}", self.id, TypeId::of::<T>(), (*value_any).type_id());
        unsafe { value_any.downcast_ref_unchecked::<T>() }
    }

    pub fn get_mut<'a: 'b, 'b>(&self, c: &'b mut impl VContext<'a, ViewData=ViewData>) -> &'b mut T where ViewData: 'b {
        let update_details = UpdateDetails::SetContextState {
            id: self.id.into(),
            backtrace: UpdateBacktrace::here()
        };

        let renderer = c.component().renderer();
        let (value_any, context_changes) = c
            .get_mut_context(&self.id.into())
            .unwrap_or_else(|| panic!("context with id ({:?}) not found in parent", self.id));
        assert!(value_any.is::<T>(), "context with id ({:?}) has wrong type: expected {:?}, got {:?}", self.id, TypeId::of::<T>(), (*value_any).type_id());
        let value = unsafe { value_any.downcast_mut_unchecked() };

        context_changes.pending_update(update_details, renderer);
        value
    }
}

impl <T: Any, ViewData: VViewData> Clone for ContextState<T, ViewData> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            phantom: self.phantom
        }
    }
}

impl <T: Any, ViewData: VViewData> Copy for ContextState<T, ViewData> {}