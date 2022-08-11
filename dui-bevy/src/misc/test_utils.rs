use std::sync::atomic::AtomicBool;

static LOGGED_ERROR: AtomicBool = AtomicBool::new(false);

pub macro error($($t:tt)*) { {
    log::error!($($t)*);
    LOGGED_ERROR.store(true, std::sync::atomic::Ordering::Release);
} }

pub macro catch_and_error($e:expr, $msg:literal $(, $args:expr)*) {
    match $e {
        Ok(v) => Some(v),
        Err(e) => {
            error!(concat!($msg, ": {}") $(, $args)*, e);
            None
        }
    }
}

pub fn check_logged_errors(fun: fn()) {
    LOGGED_ERROR.store(false, std::sync::atomic::Ordering::Release);
    fun();
    if LOGGED_ERROR.load(std::sync::atomic::Ordering::Acquire) {
        panic!("errors reported, see log")
    }
}