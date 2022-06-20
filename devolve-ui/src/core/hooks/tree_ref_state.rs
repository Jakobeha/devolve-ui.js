//! A state which can be shared across threads and outlive the current scope and is mutable outside the component.
//! Furthermore, this state tracks which components access specific properties, so when it modifies
//! those properties it can update only the needed components and not others.
//!
//! Unlike `State`, this actually contains a reference to the state itself and not just an index
//! into the component, which is `Send`. Like `State`, getting a mutable reference
//! via `TreeRefState::get_mut` (or `TreeRefState::try_get_mut`) will cause the state to update
//! the next time it's rendered. However, you can get mutable references to properties,
//! which only cause child components which access those properties to update.
//!
//! Internally this uses a mutex, so accessing the state can block and returns a `LockResult`.
//! You can use `try_get` methods to avoid blocking.
//!
//! This type is particularly useful when you want the component to trigger an effect on another thread or async context
//! (e.g. file read), then get back to the main context with the result.
//!
//! Unfortunately this state doesn't implement `Copy` because it uses reference counting.

use std::any::Any;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Drop};
use std::sync::{Arc, LockResult, Mutex, MutexGuard, TryLockResult};
use crate::core::renderer::stale_data::NeedsUpdateFlag;
use crate::core::component::context::VComponentContext;
use crate::core::component::path::{VComponentPath, VComponentRefResolved};
use crate::core::component::update_details::UpdateDetails;
use crate::core::data::obs_ref::{ObsRef, ObsRefableRoot};
use crate::core::hooks::state_internal::use_non_updating_state;
use crate::core::misc::map_lock_result::MappableLockResult;
use crate::core::view::view::VViewData;

#[derive(Debug)]
pub struct TreeRefState<T: ObsRefableRoot<VComponentPath>, ViewData: VViewData>(Arc<Mutex<T::ObsRefImpl>>, PhantomData<ViewData>);

#[derive(Debug)]
pub struct TreeAccess<'a, T: ObsRefableRoot<VComponentPath>, ViewData: VViewData>(MutexGuard<'a, T::ObsRefImpl>, PhantomData<ViewData>);

pub fn use_tree_ref_state<'a, 'a0: 'a, T: ObsRefableRoot<VComponentPath>, ViewData: VViewData + 'a>(
    c: &mut impl VComponentContext<'a, 'a0, ViewData=ViewData>,
    get_initial: impl FnOnce() -> T
) -> TreeRefState<T, ViewData> {
    let renderer = c.component_imm().renderer();
    let state = use_non_updating_state(c, || {
        let obs_ref = get_initial().into_obs_ref();
        obs_ref.after_mutate(Box::new(move |_root, referenced_paths, triggered_path| {
            if let Some(renderer) = renderer.upgrade() {
                for referenced_path in referenced_paths {
                    renderer.with_component(referenced_path, |referenced_component| {
                        if let Some(VComponentRefResolved { parent_contexts, component: referenced_component }) = referenced_component {
                            let update_details = UpdateDetails::SetTreeState {
                                // TODO: Remove allocation? (also from observer, in debug mode)
                                origin: triggered_path.to_owned()
                            };
                            referenced_component.head.update(update_details);
                            referenced_component.update(parent_contexts.collect());
                        }
                    })
                }
            }
        }));
        Arc::new(Mutex::new(get_initial().into_obs_ref(c.component_imm().renderer())))
    });

    TreeRefState(
        state.get(c).clone(),
        PhantomData
    )
}

impl <T: Any, ViewData: VViewData> TreeRefState<T, ViewData> {
    pub fn get(&self) -> LockResult<TreeAccess<'_, T, ViewData>> {
        self.0.lock().map2(TreeAccess::new)
    }

    pub fn try_get(&self) -> TryLockResult<Tree<'_, T, ViewData>> {
        self.0.try_lock().map2(TreeAccess::new)
    }
}

impl <'a, T: Any, ViewData: VViewData> TreeAccess<'a, T, ViewData> {
    fn new(inner: MutexGuard<'a, T>) -> TreeAccess<'a, T, ViewData> {
        TreeAccess(inner, PhantomData)
    }
}

impl <'a, T: Any, ViewData: VViewData> Deref for TreeAccess<'a, T, ViewData> {
    type Target = <MutexGuard<'a, T> as Deref>::Target;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl <'a, T: Any, ViewData: VViewData> DerefMut for TreeAccess<'a, T, ViewData> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}

impl <T: Any, ViewData: VViewData> Clone for TreeRefState<T, ViewData> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1)
    }

    fn clone_from(&mut self, source: Self) {
        self.0.clone_from(&source.0);
        // No-op
        self.1.clone_from(&source.1);
    }
}