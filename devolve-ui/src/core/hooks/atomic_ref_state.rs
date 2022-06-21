//! A state which can be shared across threads and outlive the current scope and is mutable outside the component.
//! Unlike `State`, this actually contains a reference to the state itself and not just an index
//! into the component, which is `Send`. Like `State`, getting a mutable reference
//! via `AtomicRefState::get_mut` (or `AtomicRefState::try_get_mut`) will cause the state to update
//! the next time it's rendered.
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
use crate::core::component::update_details::{UpdateBacktrace, UpdateDetails};
use crate::core::hooks::state_internal::use_non_updating_state;
use crate::core::misc::map_lock_result::MappableLockResult;
use crate::core::view::view::VViewData;

#[derive(Debug)]
pub struct AtomicRefState<T: Any, ViewData: VViewData> {
    data: Arc<Mutex<T>>,
    index: usize,
    flag: NeedsUpdateFlag,
    phantom: PhantomData<ViewData>
}

#[derive(Debug)]
pub struct AtomicAccess<'a, T: Any, ViewData: VViewData> {
    data: MutexGuard<'a, T>,
    phantom: PhantomData<ViewData>
}

#[derive(Debug)]
pub struct AtomicAccessMut<'a, T: Any, ViewData: VViewData> {
    data: MutexGuard<'a, T>,
    index: usize,
    flag: NeedsUpdateFlag,
    phantom: PhantomData<ViewData>
}

pub fn use_atomic_ref_state<'a, 'a0: 'a, T: Any, ViewData: VViewData + 'a>(
    c: &mut impl VComponentContext<'a, 'a0, ViewData=ViewData>,
    get_initial: impl FnOnce() -> T
) -> AtomicRefState<T, ViewData> {
    let state = use_non_updating_state(c, || Arc::new(Mutex::new(get_initial())));
    let flag = c.component().needs_update_flag();
    AtomicRefState {
        data: state.get(c).clone(),
        index: state.index,
        flag,
        phantom: PhantomData
    }
}

impl <T: Any, ViewData: VViewData> AtomicRefState<T, ViewData> {
    pub fn get(&self) -> LockResult<AtomicAccess<'_, T, ViewData>> {
        self.0.lock().map2(AtomicAccess::new)
    }

    pub fn try_get(&self) -> TryLockResult<AtomicAccess<'_, T, ViewData>> {
        self.0.try_lock().map2(AtomicAccess::new)
    }

    pub fn get_mut(&self) -> LockResult<AtomicAccessMut<'_, T, ViewData>> {
        self.0.lock().map2(|v| AtomicAccessMut::new(v, self.index, self.1.clone()))
    }

    pub fn try_get_mut(&self) -> TryLockResult<AtomicAccessMut<'_, T, ViewData>> {
        self.0.try_lock().map2(|v| AtomicAccessMut::new(v, self.index, self.1.clone()))
    }
}

impl <'a, T: Any, ViewData: VViewData> AtomicAccess<'a, T, ViewData> {
    fn new(inner: MutexGuard<'a, T>) -> AtomicAccess<'a, T, ViewData> {
        AtomicAccess {
            data: inner,
            phantom: PhantomData
        }
    }
}

impl <'a, T: Any, ViewData: VViewData> AtomicAccessMut<'a, T, ViewData> {
    fn new(inner: MutexGuard<'a, T>, index: usize, flag: NeedsUpdateFlag) -> AtomicAccessMut<'a, T, ViewData> {
        AtomicAccessMut {
            data: inner,
            index,
            flag,
            phantom: PhantomData
        }
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
        let details = UpdateDetails::SetAtomicState {
            index: self.index,
            backtrace: UpdateBacktrace::here()
        };
        let result = self.1.set(details);
        if result.is_err() {
            eprintln!("error updating from AtomicRefState: {:?}", result.unwrap_err());
        }
    }
}

impl <T: Any, ViewData: VViewData> Clone for AtomicRefState<T, ViewData> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            index: self.index,
            flag: self.flag.clone(),
            phantom: self.phantom
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.data.clone_from(&source.data);
        self.index = source.index;
        self.flag.clone_from(&source.flag);
        // No-op
        self.phantom = source.phantom;
    }
}