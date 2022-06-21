//! Generic types which are the thread-safe or thread-unsafe equivalents depending on the generic

use std::cell::{RefCell, RefMut};
use std::fmt::Debug;
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};
use std::sync::{LockResult, Mutex, MutexGuard};
use crate::core::misc::map_lock_result::MappableLockResult;
use crate::core::misc::notify_flag::{NotifyFlag, NotifyFlagTl};

pub union TSMutex<T, const IS_THREAD_SAFE: bool> {
    no: ManuallyDrop<RefCell<T>>,
    yes: ManuallyDrop<Mutex<T>>
}

pub union TSNotifyFlag<const IS_THREAD_SAFE: bool> {
    no: ManuallyDrop<NotifyFlagTl>,
    yes: ManuallyDrop<NotifyFlag>
}

pub union TSMutexGuard<'a, T, const IS_THREAD_SAFE: bool> {
    no: ManuallyDrop<RefMut<'a, T>>,
    yes: ManuallyDrop<MutexGuard<'a, T>>
}

impl <T, const IS_THREAD_SAFE: bool> TSMutex<T, IS_THREAD_SAFE> {
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

impl <T, const IS_THREAD_SAFE: bool> Debug for TSMutex<T, IS_THREAD_SAFE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match IS_THREAD_SAFE {
            true => self.yes.fmt(f),
            false => self.no.fmt(f)
        }
    }
}

impl <T, const IS_THREAD_SAFE: bool> Default for TSMutex<T, IS_THREAD_SAFE> {
    fn default() -> Self {
        match IS_THREAD_SAFE {
            true => Self { yes: ManuallyDrop::new(Mutex::default()) },
            false => Self { no: ManuallyDrop::new(RefCell::default()) }
        }
    }
}
