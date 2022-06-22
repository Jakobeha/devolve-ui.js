//! Generic types which are the thread-safe or thread-unsafe equivalents depending on the generic

use std::cell::{Ref, RefCell, RefMut};
use std::fmt::{Debug, Display, Pointer};
use std::marker::Unsize;
use std::mem::ManuallyDrop;
use std::ops::{CoerceUnsized, Deref, DerefMut, DispatchFromDyn};
use std::rc::{Rc, Weak};
use std::sync::{Arc, Weak as WeakArc, LockResult, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use crate::core::misc::map_lock_result::MappableLockResult;
use crate::core::misc::notify_flag::{NotifyFlag, NotifyFlagTl};

// region data types
pub union TSRc<T: ?Sized, const IS_THREAD_SAFE: bool> {
    no: ManuallyDrop<Rc<T>>,
    yes: ManuallyDrop<Arc<T>>
}

pub union TSWeak<T: ?Sized, const IS_THREAD_SAFE: bool> {
    no: ManuallyDrop<Weak<T>>,
    yes: ManuallyDrop<WeakArc<T>>
}

pub union TSMutex<T, const IS_THREAD_SAFE: bool> {
    no: ManuallyDrop<RefCell<T>>,
    yes: ManuallyDrop<Mutex<T>>
}

pub union TSRwLock<T, const IS_THREAD_SAFE: bool> {
    no: ManuallyDrop<RefCell<T>>,
    yes: ManuallyDrop<RwLock<T>>
}

pub union TSNotifyFlag<const IS_THREAD_SAFE: bool> {
    no: ManuallyDrop<NotifyFlagTl>,
    yes: ManuallyDrop<NotifyFlag>
}

pub union TSMutexGuard<'a, T, const IS_THREAD_SAFE: bool> {
    no: ManuallyDrop<RefMut<'a, T>>,
    yes: ManuallyDrop<MutexGuard<'a, T>>
}

pub union TSRwLockReadGuard<'a, T, const IS_THREAD_SAFE: bool> {
    no: ManuallyDrop<Ref<'a, T>>,
    yes: ManuallyDrop<RwLockReadGuard<'a, T>>
}

pub union TSRwLockWriteGuard<'a, T, const IS_THREAD_SAFE: bool> {
    no: ManuallyDrop<RefMut<'a, T>>,
    yes: ManuallyDrop<RwLockWriteGuard<'a, T>>
}
// endregion

// region unsafe send / sync impls
unsafe impl <T: Send> Send for TSRc<T, true> {}
unsafe impl <T: Sync> Sync for TSRc<T, true> {}
unsafe impl <T: Send> Send for TSWeak<T, true> {}
unsafe impl <T: Sync> Sync for TSWeak<T, true> {}
unsafe impl <T: Send> Send for TSRwLock<T, true> {}
unsafe impl <T: Send> Sync for TSRwLock<T, true> {}
unsafe impl <T: Send> Send for TSMutex<T, true> {}
unsafe impl <T: Send> Sync for TSMutex<T, true> {}
unsafe impl Send for TSNotifyFlag<true> {}
unsafe impl Sync for TSNotifyFlag<true> {}
// endregion

// region impls
impl <T, const IS_THREAD_SAFE: bool> TSRc<T, IS_THREAD_SAFE> {
    pub fn new(value: T) -> Self {
        match IS_THREAD_SAFE {
            true => Self { yes: ManuallyDrop::new(Arc::new(value)) },
            false => Self { no: ManuallyDrop::new(Rc::new(value)) }
        }
    }
}

impl <T: ?Sized, const IS_THREAD_SAFE: bool> TSRc<T, IS_THREAD_SAFE> {
    pub fn downgrade(this: &Self) -> TSWeak<T, IS_THREAD_SAFE> {
        match IS_THREAD_SAFE {
            true => TSWeak { yes: ManuallyDrop::new(Arc::downgrade(&this.yes)) },
            false => TSWeak { no: ManuallyDrop::new(Rc::downgrade(&this.no)) }
        }
    }

    pub fn strong_count(this: &Self) -> usize {
        match IS_THREAD_SAFE {
            true => Arc::strong_count(&this.yes),
            false => Rc::strong_count(&this.no)
        }
    }

    pub fn weak_count(this: &Self) -> usize {
        match IS_THREAD_SAFE {
            true => Arc::weak_count(&this.yes),
            false => Rc::weak_count(&this.no)
        }
    }
}

impl <T: ?Sized, const IS_THREAD_SAFE: bool> TSWeak<T, IS_THREAD_SAFE> {
    pub fn new() -> Self {
        match IS_THREAD_SAFE {
            true => Self { yes: ManuallyDrop::new(WeakArc::new()) },
            false => Self { no: ManuallyDrop::new(Weak::new()) }
        }
    }

    pub fn upgrade(&self) -> Option<TSRc<T, IS_THREAD_SAFE>> {
        match IS_THREAD_SAFE {
            true => self.yes.upgrade().map(|upgraded| TSRc { yes: ManuallyDrop::new(upgraded) }),
            false => self.no.upgrade().map(|upgraded| TSRc { no: ManuallyDrop::new(upgraded) })
        }
    }

    pub fn strong_count(&self) -> usize {
        match IS_THREAD_SAFE {
            true => WeakArc::strong_count(&self.yes),
            false => Weak::strong_count(&self.no)
        }
    }

    pub fn weak_count(&self) -> usize {
        match IS_THREAD_SAFE {
            true => WeakArc::weak_count(&self.yes),
            false => Weak::weak_count(&self.no)
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

    pub fn lock(&self) -> LockResult<TSMutexGuard<'_, T, IS_THREAD_SAFE>> {
        match IS_THREAD_SAFE {
            true => self.yes.lock().map2(|guard| TSMutexGuard { yes: ManuallyDrop::new(guard) }),
            false => Ok(TSMutexGuard { no: ManuallyDrop::new(self.no.borrow_mut()) })
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

    pub fn read(&self) -> LockResult<TSRwLockReadGuard<'_, T, IS_THREAD_SAFE>> {
        match IS_THREAD_SAFE {
            true => self.yes.read().map2(|guard| TSRwLockReadGuard { yes: ManuallyDrop::new(guard) }),
            false => Ok(TSRwLockReadGuard { no: ManuallyDrop::new(self.no.borrow()) })
        }
    }

    pub fn write(&self) -> LockResult<TSRwLockWriteGuard<'_, T, IS_THREAD_SAFE>> {
        match IS_THREAD_SAFE {
            true => self.yes.write().map2(|guard| TSRwLockWriteGuard { yes: ManuallyDrop::new(guard) }),
            false => Ok(TSRwLockWriteGuard { no: ManuallyDrop::new(self.no.borrow_mut()) })
        }
    }
}

impl <const IS_THREAD_SAFE: bool> TSNotifyFlag<IS_THREAD_SAFE> {
    pub fn new() -> Self {
        match IS_THREAD_SAFE {
            true => Self { yes: ManuallyDrop::new(NotifyFlag::new()) },
            false => Self { no: ManuallyDrop::new(NotifyFlagTl::new()) }
        }
    }

    pub fn set(&self) {
        match IS_THREAD_SAFE {
            true => self.yes.set(),
            false => self.no.set()
        }
    }

    pub fn get(&self) -> bool {
        match IS_THREAD_SAFE {
            true => self.yes.get(),
            false => self.no.get()
        }
    }

    pub(crate) fn clear(&self) -> bool {
        match IS_THREAD_SAFE {
            true => self.yes.clear(),
            false => self.no.clear()
        }
    }
}
// endregion

// region coerce-unsized impls
impl <T: ?Sized + Unsize<U>, U: ?Sized, const IS_THREAD_SAFE: bool> CoerceUnsized<TsRc<U, IS_THREAD_SAFE>> for TsRc<T, IS_THREAD_SAFE> {}
#[unstable(feature = "dispatch_from_dyn", issue = "none")]
impl <T: ?Sized + Unsize<U>, U: ?Sized, const IS_THREAD_SAFE: bool> DispatchFromDyn<TsRc<U, IS_THREAD_SAFE>> for TsRc<T, IS_THREAD_SAFE> {}
impl <T: ?Sized + Unsize<U>, U: ?Sized, const IS_THREAD_SAFE: bool> CoerceUnsized<TsWeak<U, IS_THREAD_SAFE>> for TsWeak<T, IS_THREAD_SAFE> {}
#[unstable(feature = "dispatch_from_dyn", issue = "none")]
impl <T: ?Sized + Unsize<U>, U: ?Sized, const IS_THREAD_SAFE: bool> DispatchFromDyn<TsWeak<U, IS_THREAD_SAFE>> for TsWeak<T, IS_THREAD_SAFE> {}
// endregion

// region pointer impls
impl <T: ?Sized, const IS_THREAD_SAFE: bool> Pointer for TSRc<T, IS_THREAD_SAFE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match IS_THREAD_SAFE {
            true => Pointer::fmt(&self.yes, f),
            false => Pointer::fmt(&self.no, f)
        }
    }
}
// endregion

// region deref impls
impl <T: ?Sized, const IS_THREAD_SAFE: bool> Deref for TSRc<T, IS_THREAD_SAFE> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match IS_THREAD_SAFE {
            true => self.yes.deref(),
            false => self.no.deref()
        }
    }
}

impl <'a, T, const IS_THREAD_SAFE: bool> Deref for TSMutexGuard<'a, T, IS_THREAD_SAFE> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match IS_THREAD_SAFE {
            true => self.yes.deref(),
            false => self.no.deref()
        }
    }
}

impl <'a, T, const IS_THREAD_SAFE: bool> DerefMut for TSMutexGuard<'a, T, IS_THREAD_SAFE> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match IS_THREAD_SAFE {
            true => self.yes.deref_mut(),
            false => self.no.deref_mut()
        }
    }
}

impl <'a, T, const IS_THREAD_SAFE: bool> Deref for TSRwLockReadGuard<'a, T, IS_THREAD_SAFE> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match IS_THREAD_SAFE {
            true => self.yes.deref(),
            false => self.no.deref()
        }
    }
}

impl <'a, T, const IS_THREAD_SAFE: bool> Deref for TSRwLockWriteGuard<'a, T, IS_THREAD_SAFE> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match IS_THREAD_SAFE {
            true => self.yes.deref(),
            false => self.no.deref()
        }
    }
}

impl <'a, T, const IS_THREAD_SAFE: bool> DerefMut for TSRwLockWriteGuard<'a, T, IS_THREAD_SAFE> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match IS_THREAD_SAFE {
            true => self.yes.deref_mut(),
            false => self.no.deref_mut()
        }
    }
}
// endregion

// region drop impls
impl <T: ?Sized, const IS_THREAD_SAFE: bool> Drop for TSRc<T, IS_THREAD_SAFE> {
    fn drop(&mut self) {
        unsafe {
            match IS_THREAD_SAFE {
                true => ManuallyDrop::drop(&mut self.yes),
                false => ManuallyDrop::drop(&mut self.no)
            }
        }
    }
}

impl <T: ?Sized, const IS_THREAD_SAFE: bool> Drop for TSWeak<T, IS_THREAD_SAFE> {
    fn drop(&mut self) {
        unsafe {
            match IS_THREAD_SAFE {
                true => ManuallyDrop::drop(&mut self.yes),
                false => ManuallyDrop::drop(&mut self.no)
            }
        }
    }
}

impl <T, const IS_THREAD_SAFE: bool> Drop for TSMutex<T, IS_THREAD_SAFE> {
    fn drop(&mut self) {
        unsafe {
            match IS_THREAD_SAFE {
                true => ManuallyDrop::drop(&mut self.yes),
                false => ManuallyDrop::drop(&mut self.no)
            }
        }
    }
}

impl <T, const IS_THREAD_SAFE: bool> Drop for TSRwLock<T, IS_THREAD_SAFE> {
    fn drop(&mut self) {
        unsafe {
            match IS_THREAD_SAFE {
                true => ManuallyDrop::drop(&mut self.yes),
                false => ManuallyDrop::drop(&mut self.no)
            }
        }
    }
}

impl <'a, T, const IS_THREAD_SAFE: bool> Drop for TSMutexGuard<'a, T, IS_THREAD_SAFE> {
    fn drop(&mut self) {
        unsafe {
            match IS_THREAD_SAFE {
                true => ManuallyDrop::drop(&mut self.yes),
                false => ManuallyDrop::drop(&mut self.no)
            }
        }
    }
}

impl <'a, T, const IS_THREAD_SAFE: bool> Drop for TSRwLockReadGuard<'a, T, IS_THREAD_SAFE> {
    fn drop(&mut self) {
        unsafe {
            match IS_THREAD_SAFE {
                true => ManuallyDrop::drop(&mut self.yes),
                false => ManuallyDrop::drop(&mut self.no)
            }
        }
    }
}

impl <'a, T, const IS_THREAD_SAFE: bool> Drop for TSRwLockWriteGuard<'a, T, IS_THREAD_SAFE> {
    fn drop(&mut self) {
        unsafe {
            match IS_THREAD_SAFE {
                true => ManuallyDrop::drop(&mut self.yes),
                false => ManuallyDrop::drop(&mut self.no)
            }
        }
    }
}

impl <const IS_THREAD_SAFE: bool> Drop for TSNotifyFlag<IS_THREAD_SAFE> {
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
impl <T: Debug + ?Sized, const IS_THREAD_SAFE: bool> Debug for TSRc<T, IS_THREAD_SAFE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match IS_THREAD_SAFE {
            true => f.debug_tuple("TSRc").field(&self.yes).finish(),
            false => f.debug_tuple("TSRc").field(&self.no).finish(),
        }
    }
}

impl <T: Debug + ?Sized, const IS_THREAD_SAFE: bool> Debug for TSWeak<T, IS_THREAD_SAFE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match IS_THREAD_SAFE {
            true => f.debug_tuple("TSWeak").field(&self.yes).finish(),
            false => f.debug_tuple("TSWeak").field(&self.no).finish(),
        }
    }
}

impl <T: Debug, const IS_THREAD_SAFE: bool> Debug for TSMutex<T, IS_THREAD_SAFE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match IS_THREAD_SAFE {
            true => f.debug_tuple("TSMutex").field(&self.yes).finish(),
            false => f.debug_tuple("TSMutex").field(&self.no).finish(),
        }
    }
}

impl <T: Debug, const IS_THREAD_SAFE: bool> Debug for TSRwLock<T, IS_THREAD_SAFE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match IS_THREAD_SAFE {
            true => f.debug_tuple("TSRwLock").field(&self.yes).finish(),
            false => f.debug_tuple("TSRwLock").field(&self.no).finish(),
        }
    }
}

impl <const IS_THREAD_SAFE: bool> Debug for TSNotifyFlag<IS_THREAD_SAFE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match IS_THREAD_SAFE {
            true => f.debug_tuple("TSNotifyFlag").field(&self.yes).finish(),
            false => f.debug_tuple("TSNotifyFlag").field(&self.no).finish(),
        }
    }
}

impl <T: Default + ?Sized, const IS_THREAD_SAFE: bool> Default for TSRc<T, IS_THREAD_SAFE> {
    fn default() -> Self {
        match IS_THREAD_SAFE {
            true => Self { yes: Default::default() },
            false => Self { no: Default::default() }
        }
    }
}

impl <T: ?Sized, const IS_THREAD_SAFE: bool> Default for TSWeak<T, IS_THREAD_SAFE> {
    fn default() -> Self {
        match IS_THREAD_SAFE {
            true => Self { yes: Default::default() },
            false => Self { no: Default::default() }
        }
    }
}

impl <T: Default, const IS_THREAD_SAFE: bool> Default for TSMutex<T, IS_THREAD_SAFE> {
    fn default() -> Self {
        match IS_THREAD_SAFE {
            true => Self { yes: Default::default() },
            false => Self { no: Default::default() }
        }
    }
}

impl <T: Default, const IS_THREAD_SAFE: bool> Default for TSRwLock<T, IS_THREAD_SAFE> {
    fn default() -> Self {
        match IS_THREAD_SAFE {
            true => Self { yes: Default::default() },
            false => Self { no: Default::default() }
        }
    }
}

impl <T: ?Sized + Display, const IS_THREAD_SAFE: bool> Display for TSRc<T, IS_THREAD_SAFE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match IS_THREAD_SAFE {
            true => Display::fmt(&self.yes, f),
            false => Display::fmt(&self.no, f)
        }
    }
}

impl <T: Display, const IS_THREAD_SAFE: bool> Display for TSMutex<T, IS_THREAD_SAFE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match IS_THREAD_SAFE {
            true => Display::fmt(&self.yes, f),
            false => Display::fmt(&self.no, f)
        }
    }
}

impl <T: Display, const IS_THREAD_SAFE: bool> Display for TSRwLock<T, IS_THREAD_SAFE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match IS_THREAD_SAFE {
            true => Display::fmt(&self.yes, f),
            false => Display::fmt(&self.no, f)
        }
    }
}

impl <T: ?Sized, const IS_THREAD_SAFE: bool> Clone for TSRc<T, IS_THREAD_SAFE> {
    fn clone(&self) -> Self {
        match IS_THREAD_SAFE {
            true => Self { yes: self.yes.clone() },
            false => Self { no: self.no.clone() }
        }
    }
}

impl <T: ?Sized, const IS_THREAD_SAFE: bool> Clone for TSWeak<T, IS_THREAD_SAFE> {
    fn clone(&self) -> Self {
        match IS_THREAD_SAFE {
            true => Self { yes: self.yes.clone() },
            false => Self { no: self.no.clone() }
        }
    }
}

impl <T: Clone, const IS_THREAD_SAFE: bool> Clone for TSMutex<T, IS_THREAD_SAFE> {
    fn clone(&self) -> Self {
        match IS_THREAD_SAFE {
            true => Self { yes: self.yes.clone() },
            false => Self { no: self.no.clone() }
        }
    }
}

impl <T: Clone, const IS_THREAD_SAFE: bool> Clone for TSRwLock<T, IS_THREAD_SAFE> {
    fn clone(&self) -> Self {
        match IS_THREAD_SAFE {
            true => Self { yes: self.yes.clone() },
            false => Self { no: self.no.clone() }
        }
    }
}
// endregion
