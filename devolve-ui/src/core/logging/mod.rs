//! Logging in devolve-ui doubles as a way to record and replay UI interactions,
//! and adjust layouts.
//!
//! There are 2 separate loggers, for logging updates (state changes, effects) and rerenders.
//! The logs are combined using timestamps as devolve-ui runs on a single thread.

pub mod common;
//! Logs component updates
pub mod update_logger;
//! Logs rerenders
pub mod render_logger;