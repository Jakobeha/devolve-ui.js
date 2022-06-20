//! Observable tree structures: when you modify a nested property on the structure,
//! it runs subscribed observers with info on the modification.
//!
//! You can derive `ObsRefable` on your data-structures, and use `into_obs_ref`
//! to convert them into obserable tree values.
//!
//! TODO: Example

/// Single-threaded
pub mod st;
/// Thread-safe
pub mod mt;
