//! `use_...` hooks which provide state, effects, and events in components.
//! See [hooks in React](https://reactjs.org/docs/hooks-intro.html) for more information.

/// State not mutable by children
pub mod state;
/// State mutable by children. Can be explicitly or implicitly passed to children
pub mod context;
/// State mutable by children and other threads.
pub mod atomic_ref_state;
/// State mutable by children and other threads. Does precise updates
pub mod tree_ref_state;
/// Effects which can be run at certain points in the component's lifecycle and based on dependencies
pub mod effect;
/// Listeners for time and input events
pub mod event;
/// Non-updating state which doesn't trigger updates
mod state_internal;
