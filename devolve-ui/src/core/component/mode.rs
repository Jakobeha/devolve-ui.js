//! Settings you can set for all components, even among different registers.
//! These are really general config: currently debug mode, and the maximum recursive updates before we detect a loop and panic.

use std::cell::RefCell;

pub struct VMode {
    is_debug: bool,
    max_recursive_updates_before_loop_detected: usize,
    #[cfg(feature = "logging")]
    is_logging: bool
}

impl VMode {
    pub fn is_debug() -> bool {
        MODE.with(|mode: &RefCell<VMode>| mode.borrow().is_debug)
    }

    pub fn set_is_debug(is_debug: bool) {
        MODE.with(|mode| mode.borrow_mut().is_debug = is_debug)
    }

    pub fn max_recursive_updates_before_loop_detected() -> usize {
        MODE.with(|mode| mode.borrow().max_recursive_updates_before_loop_detected)
    }

    pub fn set_max_recursive_updates_before_loop_detected(max_recursive_updates_before_loop_detected: usize) {
        MODE.with(|mode| mode.borrow_mut().max_recursive_updates_before_loop_detected = max_recursive_updates_before_loop_detected)
    }

    pub fn is_logging() -> bool {
        MODE.with(|mode| mode.borrow().is_logging)
    }

    pub fn set_is_logging(is_logging: bool) {
        MODE.with(|mode| mode.borrow_mut().is_logging = is_logging)
    }
}

thread_local! {
    static MODE: RefCell<VMode> = RefCell::new(VMode {
        is_debug: true,
        max_recursive_updates_before_loop_detected: 100,
        #[cfg(feature = "logging")]
        is_logging: true
    });
}