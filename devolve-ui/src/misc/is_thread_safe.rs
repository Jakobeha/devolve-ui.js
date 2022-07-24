//! Generic types which are the thread-safe or thread-unsafe equivalents depending on the generic

use std::cell::{Ref, RefCell, RefMut};
use std::fmt::{Debug, Display, Pointer};
// use std::marker::Unsize;
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};
use std::rc::{Rc, Weak};
use std::sync::{Arc, Weak as WeakArc, LockResult, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use crate::misc::map_lock_result::MappableLockResult;
use crate::misc::notify_flag::{NotifyFlag, NotifyFlagTl};

// region data types
pub union TsRc<T: ?Sized, const IS_THREAD_SAFE: bool> {
    no: ManuallyDrop<Rc<T>>,
    yes: ManuallyDrop<Arc<T>>
}

pub union TsWeak<T: ?Sized, const IS_THREAD_SAFE: bool> {
    no: ManuallyDrop<Weak<T>>,
    yes: ManuallyDrop<WeakArc<T>>
}

pub union TsMutex<T, const IS_THREAD_SAFE: bool> {
    no: ManuallyDrop<RefCell<T>>,
    yes: ManuallyDrop<Mutex<T>>
}

pub union TsRwLock<T, const IS_THREAD_SAFE: bool> {
    no: ManuallyDrop<RefCell<T>>,
    yes: ManuallyDrop<RwLock<T>>
}

pub union TsNotifyFlag<const IS_THREAD_SAFE: bool> {
    no: ManuallyDrop<NotifyFlagTl>,
    yes: ManuallyDrop<NotifyFlag>
}

pub union TsMutexGuard<'a, T, const IS_THREAD_SAFE: bool> {
    no: ManuallyDrop<RefMut<'a, T>>,
    yes: ManuallyDrop<MutexGuard<'a, T>>
}

pub union TsRwLockReadGuard<'a, T, const IS_THREAD_SAFE: bool> {
    no: ManuallyDrop<Ref<'a, T>>,
    yes: ManuallyDrop<RwLockReadGuard<'a, T>>
}

pub union TsRwLockWriteGuard<'a, T, const IS_THREAD_SAFE: bool> {
    no: ManuallyDrop<RefMut<'a, T>>,
    yes: ManuallyDrop<RwLockWriteGuard<'a, T>>
}
// endregion

// region unsafe send / sync impls
// suspicious_auto_trait_impls: I'm pretty sure these are ok because, from the issue;
//   "The builtin impl for the self type of that explicit impl applies for at least one concrete type rejected by the where-clauses of the explicit impl".
//   `Send` will never normally apply to `TsRwLock`, `TsMutex`, or `TsNotifyFlag`. Also the body is empty.
//   `!Send` is also applied to the other case anyways.

unsafe impl <T: Send> Send for TsRc<T, true> {}
unsafe impl <T: Sync> Sync for TsRc<T, true> {}
unsafe impl <T: Send> Send for TsWeak<T, true> {}
unsafe impl <T: Sync> Sync for TsWeak<T, true> {}
#[allow(suspicious_auto_trait_impls)]
unsafe impl <T: Send> Send for TsRwLock<T, true> {}
unsafe impl <T: Send> Sync for TsRwLock<T, true> {}
#[allow(suspicious_auto_trait_impls)]
unsafe impl <T: Send> Send for TsMutex<T, true> {}
unsafe impl <T: Send> Sync for TsMutex<T, true> {}
#[allow(suspicious_auto_trait_impls)]
unsafe impl Send for TsNotifyFlag<true> {}
unsafe impl Sync for TsNotifyFlag<true> {}
impl <T> !Send for TsRc<T, false> {}
impl <T> !Sync for TsRc<T, false> {}
impl <T> !Send for TsWeak<T, false> {}
impl <T> !Sync for TsWeak<T, false> {}
impl <T> !Send for TsMutex<T, false> {}
impl <T> !Sync for TsMutex<T, false> {}
impl <T> !Send for TsRwLock<T, false> {}
impl <T> !Sync for TsRwLock<T, false> {}
// endregion

// region impls
impl <T, const IS_THREAD_SAFE: bool> TsRc<T, IS_THREAD_SAFE> {
    pub fn new(value: T) -> Self {
        match IS_THREAD_SAFE {
            true => Self { yes: ManuallyDrop::new(Arc::new(value)) },
            false => Self { no: ManuallyDrop::new(Rc::new(value)) }
        }
    }
}

impl <T: ?Sized, const IS_THREAD_SAFE: bool> TsRc<T, IS_THREAD_SAFE> {
    pub fn downgrade(this: &Self) -> TsWeak<T, IS_THREAD_SAFE> {
        match IS_THREAD_SAFE {
            true => TsWeak { yes: ManuallyDrop::new(Arc::downgrade(unsafe { &this.yes })) },
            false => TsWeak { no: ManuallyDrop::new(Rc::downgrade(unsafe { &this.no })) }
        }
    }

    pub fn strong_count(this: &Self) -> usize {
        match IS_THREAD_SAFE {
            true => Arc::strong_count(unsafe { &this.yes }),
            false => Rc::strong_count(unsafe { &this.no })
        }
    }

    pub fn weak_count(this: &Self) -> usize {
        match IS_THREAD_SAFE {
            true => Arc::weak_count(unsafe { &this.yes }),
            false => Rc::weak_count(unsafe { &this.no })
        }
    }
}

impl <T, const IS_THREAD_SAFE: bool> TsWeak<T, IS_THREAD_SAFE> {
    pub fn new() -> Self {
        match IS_THREAD_SAFE {
            true => Self { yes: ManuallyDrop::new(WeakArc::new()) },
            false => Self { no: ManuallyDrop::new(Weak::new()) }
        }
    }
}

impl <T: ?Sized, const IS_THREAD_SAFE: bool> TsWeak<T, IS_THREAD_SAFE> {
    pub fn upgrade(&self) -> Option<TsRc<T, IS_THREAD_SAFE>> {
        match IS_THREAD_SAFE {
            true => unsafe { &self.yes }.upgrade().map(|upgraded| TsRc { yes: ManuallyDrop::new(upgraded) }),
            false => unsafe { &self.no }.upgrade().map(|upgraded| TsRc { no: ManuallyDrop::new(upgraded) })
        }
    }

    pub fn strong_count(&self) -> usize {
        match IS_THREAD_SAFE {
            true => WeakArc::strong_count(unsafe { &self.yes }),
            false => Weak::strong_count(unsafe { &self.no })
        }
    }

    pub fn weak_count(&self) -> usize {
        match IS_THREAD_SAFE {
            true => WeakArc::weak_count(unsafe { &self.yes }),
            false => Weak::weak_count(unsafe { &self.no })
        }
    }
}

impl <T, const IS_THREAD_SAFE: bool> TsMutex<T, IS_THREAD_SAFE> {
    pub fn new(value: T) -> Self {
        match IS_THREAD_SAFE {
            true => Self { yes: ManuallyDrop::new(Mutex::new(value)) },
            false => Self { no: ManuallyDrop::new(RefCell::new(value)) }
        }
    }

    pub fn lock(&self) -> LockResult<TsMutexGuard<'_, T, IS_THREAD_SAFE>> {
        match IS_THREAD_SAFE {
            true => unsafe { &self.yes }.lock().map2(|guard| TsMutexGuard { yes: ManuallyDrop::new(guard) }),
            false => Ok(TsMutexGuard { no: ManuallyDrop::new(unsafe { self.no.borrow_mut() }) })
        }
    }
}

impl <T, const IS_THREAD_SAFE: bool> TsRwLock<T, IS_THREAD_SAFE> {
    pub fn new(value: T) -> Self {
        match IS_THREAD_SAFE {
            true => Self { yes: ManuallyDrop::new(RwLock::new(value)) },
            false => Self { no: ManuallyDrop::new(RefCell::new(value)) }
        }
    }

    pub fn read(&self) -> LockResult<TsRwLockReadGuard<'_, T, IS_THREAD_SAFE>> {
        match IS_THREAD_SAFE {
            true => unsafe { &self.yes }.read().map2(|guard| TsRwLockReadGuard { yes: ManuallyDrop::new(guard) }),
            false => Ok(TsRwLockReadGuard { no: ManuallyDrop::new(unsafe { self.no.borrow() }) })
        }
    }

    pub fn write(&self) -> LockResult<TsRwLockWriteGuard<'_, T, IS_THREAD_SAFE>> {
        match IS_THREAD_SAFE {
            true => unsafe { &self.yes }.write().map2(|guard| TsRwLockWriteGuard { yes: ManuallyDrop::new(guard) }),
            false => Ok(TsRwLockWriteGuard { no: ManuallyDrop::new(unsafe { self.no.borrow_mut() }) })
        }
    }
}

impl <const IS_THREAD_SAFE: bool> TsNotifyFlag<IS_THREAD_SAFE> {
    pub fn new() -> Self {
        match IS_THREAD_SAFE {
            true => Self { yes: ManuallyDrop::new(NotifyFlag::new()) },
            false => Self { no: ManuallyDrop::new(NotifyFlagTl::new()) }
        }
    }

    pub fn set(&self) {
        match IS_THREAD_SAFE {
            true => unsafe { self.yes.set() },
            false => unsafe { self.no.set() }
        }
    }

    pub fn get(&self) -> bool {
        match IS_THREAD_SAFE {
            true => unsafe { self.yes.get() },
            false => unsafe { self.no.get() }
        }
    }

    pub(crate) fn clear(&self) -> bool {
        match IS_THREAD_SAFE {
            true => unsafe { self.yes.clear() },
            false => unsafe { self.no.clear() }
        }
    }
}
// endregion

// region coerce-unsized impls
// impl <T: ?Sized + Unsize<U>, U: ?Sized, const IS_THREAD_SAFE: bool> CoerceUnsized<TsRc<U, IS_THREAD_SAFE>> for TsRc<T, IS_THREAD_SAFE> {}
// impl <T: ?Sized + Unsize<U>, U: ?Sized, const IS_THREAD_SAFE: bool> DispatchFromDyn<TsRc<U, IS_THREAD_SAFE>> for TsRc<T, IS_THREAD_SAFE> {}
// impl <T: ?Sized + Unsize<U>, U: ?Sized, const IS_THREAD_SAFE: bool> CoerceUnsized<TsWeak<U, IS_THREAD_SAFE>> for TsWeak<T, IS_THREAD_SAFE> {}
// impl <T: ?Sized + Unsize<U>, U: ?Sized, const IS_THREAD_SAFE: bool> DispatchFromDyn<TsWeak<U, IS_THREAD_SAFE>> for TsWeak<T, IS_THREAD_SAFE> {}
// endregion

// region pointer impls
impl <T: ?Sized, const IS_THREAD_SAFE: bool> Pointer for TsRc<T, IS_THREAD_SAFE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match IS_THREAD_SAFE {
            true => Pointer::fmt(unsafe { self.yes.deref() }, f),
            false => Pointer::fmt(unsafe { self.no.deref() }, f)
        }
    }
}
// endregion

// region deref impls
impl <T: ?Sized, const IS_THREAD_SAFE: bool> Deref for TsRc<T, IS_THREAD_SAFE> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match IS_THREAD_SAFE {
            true => unsafe { self.yes.deref() },
            false => unsafe { self.no.deref() }
        }
    }
}

impl <'a, T, const IS_THREAD_SAFE: bool> Deref for TsMutexGuard<'a, T, IS_THREAD_SAFE> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match IS_THREAD_SAFE {
            true => unsafe { self.yes.deref() },
            false => unsafe { self.no.deref() }
        }
    }
}

impl <'a, T, const IS_THREAD_SAFE: bool> DerefMut for TsMutexGuard<'a, T, IS_THREAD_SAFE> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match IS_THREAD_SAFE {
            true => unsafe { self.yes.deref_mut() },
            false => unsafe { self.no.deref_mut() }
        }
    }
}

impl <'a, T, const IS_THREAD_SAFE: bool> Deref for TsRwLockReadGuard<'a, T, IS_THREAD_SAFE> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match IS_THREAD_SAFE {
            true => unsafe { self.yes.deref() },
            false => unsafe { self.no.deref() }
        }
    }
}

impl <'a, T, const IS_THREAD_SAFE: bool> Deref for TsRwLockWriteGuard<'a, T, IS_THREAD_SAFE> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match IS_THREAD_SAFE {
            true => unsafe { self.yes.deref() },
            false => unsafe { self.no.deref() }
        }
    }
}

impl <'a, T, const IS_THREAD_SAFE: bool> DerefMut for TsRwLockWriteGuard<'a, T, IS_THREAD_SAFE> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match IS_THREAD_SAFE {
            true => unsafe { self.yes.deref_mut() },
            false => unsafe { self.no.deref_mut() }
        }
    }
}
// endregion

// region drop impls
impl <T: ?Sized, const IS_THREAD_SAFE: bool> Drop for TsRc<T, IS_THREAD_SAFE> {
    fn drop(&mut self) {
        unsafe {
            match IS_THREAD_SAFE {
                true => ManuallyDrop::drop(&mut self.yes),
                false => ManuallyDrop::drop(&mut self.no)
            }
        }
    }
}

impl <T: ?Sized, const IS_THREAD_SAFE: bool> Drop for TsWeak<T, IS_THREAD_SAFE> {
    fn drop(&mut self) {
        unsafe {
            match IS_THREAD_SAFE {
                true => ManuallyDrop::drop(&mut self.yes),
                false => ManuallyDrop::drop(&mut self.no)
            }
        }
    }
}

impl <T, const IS_THREAD_SAFE: bool> Drop for TsMutex<T, IS_THREAD_SAFE> {
    fn drop(&mut self) {
        unsafe {
            match IS_THREAD_SAFE {
                true => ManuallyDrop::drop(&mut self.yes),
                false => ManuallyDrop::drop(&mut self.no)
            }
        }
    }
}

impl <T, const IS_THREAD_SAFE: bool> Drop for TsRwLock<T, IS_THREAD_SAFE> {
    fn drop(&mut self) {
        unsafe {
            match IS_THREAD_SAFE {
                true => ManuallyDrop::drop(&mut self.yes),
                false => ManuallyDrop::drop(&mut self.no)
            }
        }
    }
}

impl <'a, T, const IS_THREAD_SAFE: bool> Drop for TsMutexGuard<'a, T, IS_THREAD_SAFE> {
    fn drop(&mut self) {
        unsafe {
            match IS_THREAD_SAFE {
                true => ManuallyDrop::drop(&mut self.yes),
                false => ManuallyDrop::drop(&mut self.no)
            }
        }
    }
}

impl <'a, T, const IS_THREAD_SAFE: bool> Drop for TsRwLockReadGuard<'a, T, IS_THREAD_SAFE> {
    fn drop(&mut self) {
        unsafe {
            match IS_THREAD_SAFE {
                true => ManuallyDrop::drop(&mut self.yes),
                false => ManuallyDrop::drop(&mut self.no)
            }
        }
    }
}

impl <'a, T, const IS_THREAD_SAFE: bool> Drop for TsRwLockWriteGuard<'a, T, IS_THREAD_SAFE> {
    fn drop(&mut self) {
        unsafe {
            match IS_THREAD_SAFE {
                true => ManuallyDrop::drop(&mut self.yes),
                false => ManuallyDrop::drop(&mut self.no)
            }
        }
    }
}

impl <const IS_THREAD_SAFE: bool> Drop for TsNotifyFlag<IS_THREAD_SAFE> {
    fn drop(&mut self) {
        unsafe {
            match IS_THREAD_SAFE {
                true => ManuallyDrop::drop(&mut self.yes),
                false => ManuallyDrop::drop(&mut self.no)
            }
        }
    }
}
// endregion

// region auto-derive impls
impl <T: Debug + ?Sized, const IS_THREAD_SAFE: bool> Debug for TsRc<T, IS_THREAD_SAFE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match IS_THREAD_SAFE {
            true => f.debug_tuple("TsRc").field(unsafe { &self.yes }).finish(),
            false => f.debug_tuple("TsRc").field(unsafe { &self.no }).finish(),
        }
    }
}

impl <T: Debug + ?Sized, const IS_THREAD_SAFE: bool> Debug for TsWeak<T, IS_THREAD_SAFE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match IS_THREAD_SAFE {
            true => f.debug_tuple("TsWeak").field(unsafe { &self.yes }).finish(),
            false => f.debug_tuple("TsWeak").field(unsafe { &self.no }).finish(),
        }
    }
}

impl <T: Debug, const IS_THREAD_SAFE: bool> Debug for TsMutex<T, IS_THREAD_SAFE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match IS_THREAD_SAFE {
            true => f.debug_tuple("TsMutex").field(unsafe { &self.yes }).finish(),
            false => f.debug_tuple("TsMutex").field(unsafe { &self.no }).finish(),
        }
    }
}

impl <T: Debug, const IS_THREAD_SAFE: bool> Debug for TsRwLock<T, IS_THREAD_SAFE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match IS_THREAD_SAFE {
            true => f.debug_tuple("TsRwLock").field(unsafe { &self.yes }).finish(),
            false => f.debug_tuple("TsRwLock").field(unsafe { &self.no }).finish(),
        }
    }
}

impl <const IS_THREAD_SAFE: bool> Debug for TsNotifyFlag<IS_THREAD_SAFE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match IS_THREAD_SAFE {
            true => f.debug_tuple("TsNotifyFlag").field(unsafe { &self.yes }).finish(),
            false => f.debug_tuple("TsNotifyFlag").field(unsafe { &self.no }).finish(),
        }
    }
}

impl <T: Default + ?Sized, const IS_THREAD_SAFE: bool> Default for TsRc<T, IS_THREAD_SAFE> {
    fn default() -> Self {
        match IS_THREAD_SAFE {
            true => Self { yes: Default::default() },
            false => Self { no: Default::default() }
        }
    }
}

impl <T, const IS_THREAD_SAFE: bool> Default for TsWeak<T, IS_THREAD_SAFE> {
    fn default() -> Self {
        match IS_THREAD_SAFE {
            true => Self { yes: Default::default() },
            false => Self { no: Default::default() }
        }
    }
}

impl <T: Default, const IS_THREAD_SAFE: bool> Default for TsMutex<T, IS_THREAD_SAFE> {
    fn default() -> Self {
        match IS_THREAD_SAFE {
            true => Self { yes: Default::default() },
            false => Self { no: Default::default() }
        }
    }
}

impl <T: Default, const IS_THREAD_SAFE: bool> Default for TsRwLock<T, IS_THREAD_SAFE> {
    fn default() -> Self {
        match IS_THREAD_SAFE {
            true => Self { yes: Default::default() },
            false => Self { no: Default::default() }
        }
    }
}

impl <T: ?Sized + Display, const IS_THREAD_SAFE: bool> Display for TsRc<T, IS_THREAD_SAFE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match IS_THREAD_SAFE {
            true => Display::fmt(unsafe { self.yes.deref() }, f),
            false => Display::fmt(unsafe { self.no.deref() }, f)
        }
    }
}

impl <T: ?Sized, const IS_THREAD_SAFE: bool> Clone for TsRc<T, IS_THREAD_SAFE> {
    fn clone(&self) -> Self {
        match IS_THREAD_SAFE {
            true => Self { yes: unsafe { self.yes.clone() } },
            false => Self { no: unsafe { self.no.clone() } }
        }
    }
}

impl <T: ?Sized, const IS_THREAD_SAFE: bool> Clone for TsWeak<T, IS_THREAD_SAFE> {
    fn clone(&self) -> Self {
        match IS_THREAD_SAFE {
            true => Self { yes: unsafe { self.yes.clone() } },
            false => Self { no: unsafe { self.no.clone() } }
        }
    }
}
// endregion
