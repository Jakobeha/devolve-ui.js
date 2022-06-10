///! A state which can be shared across threads and outlive the current scope.

use std::any::Any;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Drop};
use std::sync::{Arc, Weak, LockResult, Mutex, MutexGuard, TryLockResult};
use crate::core::component::component::VComponent;
use crate::core::hooks::state_internal::use_non_updating_state;
use crate::core::misc::map_lock_result::MappableLockResult;
use crate::core::misc::notify_bool::FlagForOtherThreads;
use crate::core::view::view::VViewData;

#[derive(Debug, Clone)]
pub struct AtomicRefState<T: Any, ViewData: VViewData>(Arc<Mutex<T>>, Weak<FlagForOtherThreads>, PhantomData<ViewData>);

#[derive(Debug)]
pub struct AtomicAccess<'a, T: Any, ViewData: VViewData>(MutexGuard<'a, T>, PhantomData<ViewData>);

#[derive(Debug)]
pub struct AtomicAccessMut<'a, T: Any, ViewData: VViewData>(MutexGuard<'a, T>, Weak<FlagForOtherThreads>, PhantomData<ViewData>);

pub fn use_atomic_ref_state<T: Any, ViewData: VViewData>(
    c: &mut Box<VComponent<ViewData>>,
    get_initial: impl FnOnce() -> T
) -> AtomicRefState<T, ViewData> {
    AtomicRefState(
        use_non_updating_state(c, || Arc::new(Mutex::new(get_initial()))).get(c).clone(),
        c.invalidate_flag(),
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

    pub fn get_mut(&mut self) -> LockResult<AtomicAccessMut<'_, T, ViewData>> {
        self.0.lock().map2(|v| AtomicAccessMut::new(v, self.1.clone()))
    }

    pub fn try_get_mut(&mut self) -> TryLockResult<AtomicAccessMut<'_, T, ViewData>> {
        self.0.try_lock().map2(|v| AtomicAccessMut::new(v, self.1.clone()))
    }
}

impl <'a, T: Any, ViewData: VViewData> AtomicAccess<'a, T, ViewData> {
    fn new(inner: MutexGuard<'a, T>) -> AtomicAccess<'a, T, ViewData> {
        AtomicAccess(inner, PhantomData)
    }
}

impl <'a, T: Any, ViewData: VViewData> AtomicAccessMut<'a, T, ViewData> {
    fn new(inner: MutexGuard<'a, T>, invalidate_flag: Weak<FlagForOtherThreads>) -> AtomicAccessMut<'a, T, ViewData> {
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