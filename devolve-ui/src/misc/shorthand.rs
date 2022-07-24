//! Shorter syntax for common functions.

use crate::misc::partial_default::PartialDefault;

/// Default
pub fn d<D: Default>() -> D {
    D::default()
}

pub fn pd<D: PartialDefault>(args: D::RequiredArgs) -> D {
    D::partial_default(args)
}