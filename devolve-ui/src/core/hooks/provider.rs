//! Component state which can be passed to and accessed from children.
//! By default it's also implicitly passed, however you can make this explicit with `use_provide_explicit`
//!
//! See `devolve_ui::core::hooks::state` type for more information about state.
//! This is similar except its created via `use_provide` and accessible in children `use_consume`.
//! For each type of context, you must create a static const `context` object which functions as a
//! shared key for both `use_provide` and `use_consume` and also contains the underlying type.
//! If a child calls `use_provide` with the same context as its parent, then the child's context
//! will shadow in its own children.

use std::any::{Any, TypeId};
use std::marker::{PhantomData, PhantomPinned};
use crate::core::component::context::{VComponentContext, VContext, VContextIndex};
use crate::core::component::update_details::{UpdateBacktrace, UpdateDetails};
use crate::core::view::view::VViewData;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

/// Provider id is a reference to this region in memory.
#[derive(Debug)]
pub struct ProviderIdSource<T: Any>(PhantomData<T>, PhantomPinned);

impl <T: Any> ProviderIdSource<T> {
    pub const fn new() -> Self {
        ProviderIdSource(PhantomData, PhantomPinned)
    }
}

/// Provider id with provided state type. Identifies which state to access in `use_consume`
pub type ProviderId<T> = *const ProviderIdSource<T>;

/// Untyped provider id. Identifies which state to access in `use_consume`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct UntypedProviderId(usize);

impl <T: Any> From<*const ProviderIdSource<T>> for UntypedProviderId {
    fn from(id: *const ProviderIdSource<T>) -> Self {
        UntypedProviderId(id as usize)
    }
}

#[derive(Debug)]
pub struct ProvidedState<T: Any, ViewData: VViewData> {
    id: ProviderId<T>,
    phantom: PhantomData<ViewData>
}

pub(super) fn _use_provide<'a, 'a0: 'a, T: Any, ViewData: VViewData + 'a, Ctx: VComponentContext<'a, 'a0, ViewData=ViewData>>(
    c: &mut Ctx,
    id: ProviderId<T>,
    get_initial: impl FnOnce(&mut Ctx) -> Box<T>
) -> ProvidedState<T, ViewData> {
    let local_contexts = c.local_contexts();
    if !local_contexts.contains_key(&id.into()) {
        let initial_state = get_initial(c);
        let conflict = c.local_contexts().insert(id.into(), initial_state);
        assert!(conflict.is_none(), "use_provide inside another use_provide.get_initial with the same id, what are you doing?")
    }
    ProvidedState {
        id,
        phantom: PhantomData
    }
}

pub(super) fn _use_consume<'a, 'a0: 'a, T: Any, ViewData: VViewData + 'a>(
    c: &mut impl VComponentContext<'a, 'a0, ViewData=ViewData>,
    id: ProviderId<T>
) -> ProvidedState<T, ViewData> {
    let existing = c.get_mut_context(&id.into());
    assert!(existing.is_some(), "context with id ({:?}) not found in parent", id);
    ProvidedState {
        id,
        phantom: PhantomData
    }
}

impl <T: Any, ViewData: VViewData> VContextIndex<ViewData> for ProvidedState<T, ViewData> {
    type T = T;

    fn get<'a: 'b, 'b>(&self, c: &'b impl VContext<'a, ViewData=ViewData>) -> &'b T where ViewData: 'b {
        let value_any = c
            .get_context(&self.id.into())
            .unwrap_or_else(|| panic!("context with id ({:?}) not found in parent", self.id));
        assert!(value_any.is::<T>(), "context with id ({:?}) has wrong type: expected {:?}, got {:?}", self.id, TypeId::of::<T>(), (*value_any).type_id());
        unsafe { value_any.downcast_ref_unchecked::<T>() }
    }

    fn get_mut<'a: 'b, 'b>(&self, c: &'b mut impl VContext<'a, ViewData=ViewData>) -> &'b mut T where ViewData: 'b {
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

impl <T: Any, ViewData: VViewData> Clone for ProvidedState<T, ViewData> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            phantom: self.phantom
        }
    }
}

impl <T: Any, ViewData: VViewData> Copy for ProvidedState<T, ViewData> {}