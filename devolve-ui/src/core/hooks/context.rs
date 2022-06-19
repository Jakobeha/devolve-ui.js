//! Component state which is implicitly passed to and can be accessed from children.
//!
//! See `devolve_ui::core::hooks::state` type for more information about state.
//! This is similar except its created via `use_provide` and accessible in children `use_consume`.
//! For each type of context, you must create a static const `context` object which functions as a
//! shared key for both `use_provide` and `use_consume` and also contains the underlying type.
//! If a child calls `use_provide` with the same context as its parent, then the child's context
//! will shadow in its own children.

use std::any::Any;
use std::marker::PhantomData;
use crate::core::component::context::{VComponentContext, VContext};
use crate::core::component::update_details::{UpdateBacktrace, UpdateDetails};
use crate::core::hooks::state::StateDeref;
use crate::core::view::view::VViewData;

#[derive(Debug)]
pub struct ContextIdSource<T: Any>;

pub type ContextId<T: Any> = *const ContextIdSource<T>;

pub type AnonContextId = *const ();

#[derive(Debug)]
pub struct ContextState<T: Any, ViewData: VViewData> {
    id: ContextId<T>,
    phantom: PhantomData<ViewData>
}

pub fn use_provide<'a, T: Any, ViewData: VViewData + 'a>(
    c: &mut impl VComponentContext<'a, ViewData=ViewData>,
    id: ContextId<T>,
    get_initial: impl FnOnce() -> T
) -> ContextState<T, ViewData> {
    let conflict = c.get_mut_or_insert_context(id as AnonContextId, || Box::new(get_initial()));
    assert!(conflict.is_none(), "contexts with same id ({:?}) in same component", id);
    ContextState {
        id,
        phantom: PhantomData
    }
}

pub fn use_consume<'a, T: Any, ViewData: VViewData + 'a>(
    c: &mut impl VComponentContext<'a, ViewData=ViewData>,
    id: ContextId<T>
) -> ContextState<T, ViewData> {
    let existing = c.get_mut_context(&id as &AnonContextId);
    assert!(existing.is_some(), "context with id ({:?}) not found in parent", id);
    ContextState {
        id,
        phantom: PhantomData
    }
}

impl <T: Any, ViewData: VViewData> ContextState<T, ViewData> {
    pub fn get<'a: 'b, 'b>(&self, c: &'b mut impl VContext<'a, ViewData=ViewData>) -> &'b T where ViewData: 'b {
        c.get_context(&self.id as &AnonContextId).unwrap_or_else(|| panic!("context with id ({:?}) not found in parent", self.id))
    }

    pub fn get_mut<'a: 'b, 'b>(&self, c: &'b mut impl VContext<'a, ViewData=ViewData>) -> StateDeref<'b, T, ViewData> where ViewData: 'b {
        let update_details = UpdateDetails::SetContextState {
            index: self.0.index,
            backtrace: UpdateBacktrace::her()
        };
        // See comment in StateDeref::drop
        let component = c.component() as *mut _;
        StateDeref {
            component,
            update_details,
            value: c.get_mut_context(&self.id as &AnonContextId).unwrap_or_else(|| panic!("context with id ({:?}) not found in parent", self.id))
        }
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