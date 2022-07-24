//! Random utility code which aren't really `devolve-ui` specific but needed by `devolve-ui`.

pub mod either_future;

pub fn assert_is_unpin<T: Unpin>(x: T) -> T {
    x
}
