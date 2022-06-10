use std::sync::atomic::AtomicBool;

/// An atomic bool which can be set to true and checked / cleared only in this crate.
/// Usually this is set by other threads to be checked by a `!Send + !Sync` target,
/// although it can be set from the target's thread as well.
#[derive(Debug)]
pub struct NotifyFlag(AtomicBool);

impl NotifyFlag {
    pub fn new() -> Self {
        Self(AtomicBool::new(false))
    }

    pub fn set(&self) {
        self.0.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn get(&self) -> bool {
        self.0.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub(crate) fn clear(&self) -> bool {
        self.0.swap(false, std::sync::atomic::Ordering::Relaxed)
    }
}