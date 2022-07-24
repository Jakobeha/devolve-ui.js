//! Format mode: a workaround for not having custom format specifiers or flags.
//! This is just a thread-local global which determines how certain traits are formatted.
//! It's not "Rust-y" but unless you're writing text using multiple threads, it's safe and fast.

use std::cell::Cell;
use bitflags::bitflags;

#[derive(Debug, Clone, Copy)]
pub struct FormatMode(FormatModeFlags);

bitflags! {
    pub struct FormatModeFlags: u8 {
        const DEFAULT = 0b0000;
        const REAL_DIMENSIONS = 0b0001;
    }
}

thread_local! {
    pub static FORMAT_MODE: Cell<FormatMode> = Cell::new(FormatMode(FormatModeFlags::DEFAULT));
}

impl FormatMode {
    pub fn with<R>(flags: FormatModeFlags, f: impl FnOnce() -> R) -> R {
        let old = FORMAT_MODE.with(|f| {
            let old = f.get();
            f.set(FormatMode(old.0.union(flags)));
            old
        });
        let result = f();
        FORMAT_MODE.with(|f| f.set(old));
        result
    }

    pub fn has(flags: FormatModeFlags) -> bool {
        FORMAT_MODE.with(|f| f.get().0.contains(flags))
    }
}