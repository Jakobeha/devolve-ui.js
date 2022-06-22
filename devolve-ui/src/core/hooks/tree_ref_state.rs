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

use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, LockResult, Mutex, MutexGuard, TryLockResult};
use crate::core::component::context::{VComponentContext, VContext};
use crate::core::component::path::VComponentPath;
use crate::core::component::update_details::UpdateDetails;
use crate::core::data::obs_ref::st::{ObsRef, ObsRefableRoot, SubCtx};
use crate::core::hooks::state_internal::use_non_updating_state;
use crate::core::misc::map_lock_result::MappableLockResult;
use crate::core::view::view::VViewData;

#[derive(Debug)]
pub struct TreeRefState<T: ObsRefableRoot<VContextSubCtx<ViewData>>, ViewData: VViewData + 'static>(Arc<Mutex<T::ObsRefImpl>>, PhantomData<ViewData>);

#[derive(Debug)]
pub struct TreeAccess<'a, T: ObsRefableRoot<VContextSubCtx<ViewData>>, ViewData: VViewData + 'static>(MutexGuard<'a, T::ObsRefImpl>, PhantomData<ViewData>);

pub fn use_tree_ref_state<'a, 'a0: 'a, T: ObsRefableRoot<VContextSubCtx<ViewData>>, ViewData: VViewData + 'static>(
    c: &mut impl VComponentContext<'a, 'a0, ViewData=ViewData>,
    get_initial: impl FnOnce() -> T
) -> TreeRefState<T, ViewData> {
    let component = c.component_imm();
    let notifier = component
        .renderer()
        .upgrade()
        .expect("use_tree_ref_state called in context with nil renderer")
        .needs_update_notifier();
    let state = use_non_updating_state(c, || {
        let obs_ref = get_initial().into_obs_ref();
        obs_ref.after_mutate(Box::new(move |_root, referenced_paths, triggered_path| {
            for referenced_path in referenced_paths {
                let result = notifier.set(referenced_path, UpdateDetails::SetTreeState {
                    // TODO: Remove allocation? (also from observer, in debug mode)
                    origin: triggered_path.to_owned()
                });
                if result.is_err() {
                    eprintln!("failed to set needs update flag from {} for {}", triggered_path, referenced_path);
                }
            }
        }));
        Arc::new(Mutex::new(get_initial().into_obs_ref()))
    });

    TreeRefState(
        state.get(c).clone(),
        PhantomData
    )
}

impl <T: ObsRefableRoot<VContextSubCtx<ViewData>>, ViewData: VViewData + 'static> TreeRefState<T, ViewData> {
    pub fn get(&self) -> LockResult<TreeAccess<'_, T, ViewData>> {
        self.0.lock().map2(TreeAccess::new)
    }

    pub fn try_get(&self) -> TryLockResult<TreeAccess<'_, T, ViewData>> {
        self.0.try_lock().map2(TreeAccess::new)
    }
}

impl <'a, T: ObsRefableRoot<VContextSubCtx<ViewData>>, ViewData: VViewData + 'static> TreeAccess<'a, T, ViewData> {
    fn new(inner: MutexGuard<'a, T>) -> TreeAccess<'a, T, ViewData> {
        TreeAccess(inner, PhantomData)
    }
}

impl <'a, T: ObsRefableRoot<VContextSubCtx<ViewData>>, ViewData: VViewData + 'static> Deref for TreeAccess<'a, T, ViewData> {
    type Target = <MutexGuard<'a, T> as Deref>::Target;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl <'a, T: ObsRefableRoot<VContextSubCtx<ViewData>>, ViewData: VViewData + 'static> DerefMut for TreeAccess<'a, T, ViewData> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}

impl <T: ObsRefableRoot<VContextSubCtx<ViewData>>, ViewData: VViewData + 'static> Clone for TreeRefState<T, ViewData> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1)
    }

    fn clone_from(&mut self, source: &Self) {
        self.0.clone_from(&source.0);
        // No-op
        self.1.clone_from(&source.1);
    }
}

struct VContextSubCtx<ViewData: VViewData + 'static>(PhantomData<ViewData>);

impl <ViewData: VViewData + 'static> SubCtx for VContextSubCtx<ViewData> {
    type Input<'a> = &'a dyn VContext<'a, ViewData=ViewData>;
    type Key = VComponentPath;

    fn convert_into_subscription_key(input: Self::Input<'_>) -> Self::Key {
        input.component_imm().path().clone()
    }
}