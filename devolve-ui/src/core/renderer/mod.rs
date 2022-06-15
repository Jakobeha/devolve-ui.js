//! Manages component data, events / listeners, and rendering.
//! How do we draw (output) the views? How do we handle user interaction and world state (input)?
//! This module is how. See the sub-modules for more documentation.

pub mod engine;
pub mod listeners;
pub mod render;
pub mod renderer;
pub mod running;
pub mod stale_data;
#[cfg(feature = "input")]
pub mod input;
