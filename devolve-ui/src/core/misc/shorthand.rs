//! Shorter syntax for common functions.

/// Default
pub fn d<D: Default>() -> D {
    D::default()
}