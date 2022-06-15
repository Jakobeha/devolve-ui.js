//! A state which can be shared across threads and outlive the current scope.
//! Unlike `State`, this actually contains a reference to the state itself and not just an index
//! into the component, which is `Send`. Like `State`, getting a mutable reference
//! via `AtomicRefState::get_mut` (or `AtomicRefState::try_get_mut`) will cause the state to update
//! the next time it's rendered. Internally this uses a mutex, so `get_mut` can block and returns a `LockResult`.
//!
//! This type is particularly useful when you want the component to trigger an effect on another thread or async context
//! (e.g. file read), then get back to the main context with the result.
//!
//! Unfortunately this state doesn't implement `Copy` because it uses reference counting.

use std::any::Any;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Drop};
use std::sync::{Arc, Weak, LockResult, Mutex, MutexGuard, TryLockResult};
use crate::core::renderer::stale_data::NeedsUpdateFlag;
use crate::core::component::context::VComponentContext;
use crate::core::hooks::state_internal::use_non_updating_state;
use crate::core::misc::map_lock_result::MappableLockResult;
use crate::core::view::view::VViewData;

#[derive(Debug)]
pub struct AtomicRefState<T: Any, ViewData: VViewData>(Arc<Mutex<T>>, Weak<NeedsUpdateFlag>, PhantomData<ViewData>);

#[derive(Debug)]
pub struct AtomicAccess<'a, T: Any, ViewData: VViewData>(MutexGuard<'a, T>, PhantomData<ViewData>);

#[derive(Debug)]
pub struct AtomicAccessMut<'a, T: Any, ViewData: VViewData>(MutexGuard<'a, T>, Weak<NeedsUpdateFlag>, PhantomData<ViewData>);

pub fn use_atomic_ref_state<'a, T: Any, ViewData: VViewData + 'a>(
    c: &mut impl VComponentContext<'a, ViewData=ViewData>,
    get_initial: impl FnOnce() -> T
) -> AtomicRefState<T, ViewData> {
    let state = use_non_updating_state(c, || Arc::new(Mutex::new(get_initial())));
    let invalidate_flag = c.component().invalidate_flag();
    AtomicRefState(
        state.get(c).clone(),
        invalidate_flag,
        PhantomData
    )
}

impl <T: Any, ViewData: VViewData> AtomicRefState<T, ViewData> {
    pub fn get(&self) -> LockResult<AtomicAccess<'_, T, ViewData>> {
        self.0.lock().map2(AtomicAccess::new)
    }

    pub fn try_get(&self) -> TryLockResult<AtomicAccess<'_, T, ViewData>> {
        self.0.try_lock().map2(AtomicAccess::new)
    }

    pub fn get_mut(&self) -> LockResult<AtomicAccessMut<'_, T, ViewData>> {
        self.0.lock().map2(|v| AtomicAccessMut::new(v, self.1.clone()))
    }

    pub fn try_get_mut(&self) -> TryLockResult<AtomicAccessMut<'_, T, ViewData>> {
        self.0.try_lock().map2(|v| AtomicAccessMut::new(v, self.1.clone()))
    }
}

impl <'a, T: Any, ViewData: VViewData> AtomicAccess<'a, T, ViewData> {
    fn new(inner: MutexGuard<'a, T>) -> AtomicAccess<'a, T, ViewData> {
        AtomicAccess(inner, PhantomData)
    }
}

impl <'a, T: Any, ViewData: VViewData> AtomicAccessMut<'a, T, ViewData> {
    fn new(inner: MutexGuard<'a, T>, invalidate_flag: Weak<NeedsUpdateFlag>) -> AtomicAccessMut<'a, T, ViewData> {
        AtomicAccessMut(inner, invalidate_flag, PhantomData)
    }
}

impl <'a, T: Any, ViewData: VViewData> Deref for AtomicAccess<'a, T, ViewData> {
    type Target = <MutexGuard<'a, T> as Deref>::Target;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}


impl <'a, T: Any, ViewData: VViewData> Deref for AtomicAccessMut<'a, T, ViewData> {
    type Target = <MutexGuard<'a, T> as Deref>::Target;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl <'a, T: Any, ViewData: VViewData> DerefMut for AtomicAccessMut<'a, T, ViewData> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}

impl <'a, T: Any, ViewData: VViewData> Drop for AtomicAccessMut<'a, T, ViewData> {
    fn drop(&mut self) {
        if let Some(notify_flag) = self.1.upgrade() {
            notify_flag.set();
        }
    }
}

impl <T: Any, ViewData: VViewData> Clone for AtomicRefState<T, ViewData> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone(), self.2)
    }
}