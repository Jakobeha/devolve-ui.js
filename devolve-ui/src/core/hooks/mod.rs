//! `use_...` hooks which provide state, effects, and events in components.
//! See [hooks in React](https://reactjs.org/docs/hooks-intro.html) for more information.

use std::any::Any;
use std::time::Duration;
use crate::core::component::context::{VComponentContext1, VDestructorContext2, VEffectContext2, VPlainContext2};
#[cfg(feature = "obs-ref")]
use crate::core::data::obs_ref::st::ObsRefableRoot;
use crate::core::hooks::atomic_ref_state::{_use_atomic_ref_state, AtomicRefState};
use crate::core::hooks::context::{_use_consume, _use_provide, ContextId, ContextState};
use crate::core::hooks::effect::{_use_effect, _use_effect_on_create, _use_effect_with_deps, CollectionOfPartialEqs, NoDependencies, UseEffectRerun};
#[cfg(feature = "time")]
use crate::core::hooks::event::{_use_delay, _use_interval, _use_tick_listener, _use_tick_listener_when};
#[cfg(feature = "input")]
use crate::core::hooks::event::{_use_key_listener, _use_key_listener_when, _use_mouse_listener, _use_mouse_listener_when, _use_resize_listener, _use_resize_listener_when};
use crate::core::hooks::event::CallFirst;
use crate::core::hooks::state::{_use_state, State};
#[cfg(feature = "obs-ref")]
use crate::core::hooks::tree_ref_state::{_use_tree_ref_state, TreeRefState, VContextSubCtx};
#[cfg(feature = "input")]
use crate::core::renderer::input::{KeyEvent, MouseEvent, ResizeEvent};
use crate::core::view::view::VViewData;

/// State not mutable by children
pub mod state;
/// State mutable by children. Can be explicitly or implicitly passed to children
pub mod context;
/// State mutable by children and other threads.
pub mod atomic_ref_state;
/// State mutable by children and other threads. Does precise updates
#[cfg(feature = "obs-ref")]
pub mod tree_ref_state;
/// Effects which can be run at certain points in the component's lifecycle and based on dependencies
pub mod effect;
/// Listeners for time and input events
pub mod event;
/// Non-updating state which doesn't trigger updates
mod state_internal;

pub trait BuiltinHooks <'a, 'a0: 'a, Props: Any, ViewData: VViewData + 'static> {
    fn use_state<T: Any>(&mut self, initial_state: impl FnOnce() -> T) -> State<T, ViewData>;
    fn use_provide<T: Any>(&mut self, id: ContextId<T>, get_initial: impl FnOnce() -> Box<T>) -> ContextState<T, ViewData>;
    fn use_consume<T: Any>(&mut self,id: ContextId<T>) -> ContextState<T, ViewData>;
    fn use_atomic_ref_state<T: Any>(&mut self, get_initial: impl FnOnce() -> T) -> AtomicRefState<T, ViewData>;
    fn use_tree_ref_state<T: ObsRefableRoot<VContextSubCtx<ViewData>> + 'static>(&mut self, get_initial: impl FnOnce() -> T) -> TreeRefState<T, ViewData> where ViewData: 'static;
    /// Runs a closure according to `rerun`. The closure should contain an effect,
    /// while the component's body should otherwise be a "pure" function based on its
    /// props and state hooks like `use_state`.
    fn use_effect<Destructor: FnOnce(VDestructorContext2<'_, '_, Props, ViewData>) + 'static>(
        &mut self,
        rerun: UseEffectRerun<NoDependencies>,
        effect: impl Fn(VEffectContext2<'_, '_, Props, ViewData>) -> Destructor + 'static
    );
    /// Runs a closure once on create. The closure should contain an effect,
    /// while the component's body should otherwise be a "pure" function based on its
    /// props and state hooks like `use_state`.
    ///
    /// The behavior is exactly like `use_effect` and `use_effect_with_deps` when given `UseEffectRerun::OnCreate`.
    /// However, this function allows you to pass an `FnOnce` to `effect` since we statically know it will only be called once.
    fn use_effect_on_create<Destructor: FnOnce(VDestructorContext2<'_, '_, Props, ViewData>) + 'static>(
            &mut self,
            effect: impl FnOnce(VEffectContext2<'_, '_, Props, ViewData>) -> Destructor + 'static
    );
    /// Runs a closure according to `rerun`. The closure should contain an effect,
    /// while the component's body should otherwise be a "pure" function based on its
    /// props and state hooks like `use_state`.
    ///
    /// This function is actually the exact same as `use_effect`, but exposes the dependencies as a type parameter.
    /// Without the 2 versions, you would always have to specify dependencies on `use_effect` even if the enum variant didn't have them.
    fn use_effect_with_deps<
        Dependencies: CollectionOfPartialEqs + 'static,
        Destructor: FnOnce(VDestructorContext2<'_, '_, Props, ViewData>) + 'static
    >(
        &mut self,
        rerun: UseEffectRerun<Dependencies>,
        effect: impl Fn(VEffectContext2<'_, '_, Props, ViewData>) -> Destructor + 'static
    );
    #[cfg(feature = "time")]
    fn use_tick_listener(&mut self, listener: impl Fn(VPlainContext2<'_, '_, Props, ViewData>, &Duration) + 'static);
    #[cfg(feature = "time")]
    fn use_tick_listener_when(&mut self, predicate: bool, listener: impl Fn(VPlainContext2<'_, '_, Props, ViewData>, &Duration) + 'static);
    #[cfg(feature = "input")]
    fn use_key_listener(&mut self, listener: impl Fn(VPlainContext2<'_, '_, Props, ViewData>, &KeyEvent) + 'static);
    #[cfg(feature = "input")]
    fn use_key_listener_when(&mut self, predicate: bool, listener: impl Fn(VPlainContext2<'_, '_, Props, ViewData>, &KeyEvent) + 'static);
    #[cfg(feature = "input")]
    fn use_mouse_listener(&mut self, listener: impl Fn(VPlainContext2<'_, '_, Props, ViewData>, &MouseEvent) + 'static);
    #[cfg(feature = "input")]
    fn use_mouse_listener_when(&mut self, predicate: bool, listener: impl Fn(VPlainContext2<'_, '_, Props, ViewData>, &MouseEvent) + 'static);
    #[cfg(feature = "input")]
    fn use_resize_listener(&mut self, listener: impl Fn(VPlainContext2<'_, '_, Props, ViewData>, &ResizeEvent) + 'static);
    #[cfg(feature = "input")]
    fn use_resize_listener_when(&mut self, predicate: bool, listener: impl Fn(VPlainContext2<'_, '_, Props, ViewData>, &ResizeEvent) + 'static);
    #[cfg(feature = "input")]
    fn use_interval(&mut self, interval: Duration, call_first: CallFirst, listener: impl Fn(VPlainContext2<'_, '_, Props, ViewData>) + 'static);
    #[cfg(feature = "time")]
    fn use_delay(&mut self, delay: Duration, listener: impl FnOnce(VPlainContext2<'_, '_, Props, ViewData>) + 'static);
}

impl <'a, 'a0: 'a, Props: Any, ViewData: VViewData + 'static> BuiltinHooks<'a, 'a0, Props, ViewData> for VComponentContext1<'a, 'a0, Props, ViewData> {
    fn use_state<T: Any>(&mut self, initial_state: impl FnOnce() -> T) -> State<T, ViewData> {
        _use_state(self, initial_state)
    }

    fn use_provide<T: Any>(&mut self, id: ContextId<T>, get_initial: impl FnOnce() -> Box<T>) -> ContextState<T, ViewData> {
        _use_provide(self, id, get_initial)
    }

    fn use_consume<T: Any>(&mut self, id: ContextId<T>) -> ContextState<T, ViewData> {
        _use_consume(self, id)
    }

    fn use_atomic_ref_state<T: Any>(&mut self, get_initial: impl FnOnce() -> T) -> AtomicRefState<T, ViewData> {
        _use_atomic_ref_state(self, get_initial)
    }

    fn use_tree_ref_state<T: ObsRefableRoot<VContextSubCtx<ViewData>> + 'static>(&mut self, get_initial: impl FnOnce() -> T) -> TreeRefState<T, ViewData> where ViewData: 'static {
        _use_tree_ref_state(self, get_initial)
    }

    fn use_effect<Destructor: FnOnce(VDestructorContext2<'_, '_, Props, ViewData>) + 'static>(
        &mut self,
        rerun: UseEffectRerun<NoDependencies>,
        effect: impl Fn(VEffectContext2<'_, '_, Props, ViewData>) -> Destructor + 'static
    ) {
        _use_effect(self, rerun, effect)
    }

    fn use_effect_on_create<Destructor: FnOnce(VDestructorContext2<'_, '_, Props, ViewData>) + 'static>(
        &mut self,
        effect: impl FnOnce(VEffectContext2<'_, '_, Props, ViewData>) -> Destructor + 'static
    ) {
        _use_effect_on_create(self, effect)
    }

    fn use_effect_with_deps<
        Dependencies: CollectionOfPartialEqs + 'static,
        Destructor: FnOnce(VDestructorContext2<'_, '_, Props, ViewData>) + 'static
    >(
        &mut self,
        rerun: UseEffectRerun<Dependencies>,
        effect: impl Fn(VEffectContext2<'_, '_, Props, ViewData>) -> Destructor + 'static
    ) {
        _use_effect_with_deps(self, rerun, effect)
    }

    /// Register a function which will be called every time there is a tick event.
    /// The duration is subject to delta-time diff, so use absolute intervals if you want precise time.
    #[cfg(feature = "time")]
    fn use_tick_listener(&mut self, listener: impl Fn(VPlainContext2<'_, '_, Props, ViewData>, &Duration) + 'static) {
        _use_tick_listener(self, listener)
    }

    /// Register a function which will be called every time there is a tick event.
    /// The duration is subject to delta-time diff, so use absolute intervals if you want precise time.
    #[cfg(feature = "time")]
    fn use_tick_listener_when(&mut self, predicate: bool, listener: impl Fn(VPlainContext2<'_, '_, Props, ViewData>, &Duration) + 'static) {
        _use_tick_listener_when(self, predicate, listener)
    }

    /// Register a function which will be called every time there is a key event (see `KeyEvent` for event types).
    #[cfg(feature = "input")]
    fn use_key_listener(&mut self, listener: impl Fn(VPlainContext2<'_, '_, Props, ViewData>, &KeyEvent) + 'static) {
        _use_key_listener(self, listener)
    }

    /// Register a function which will be called every time there is a key event (see `KeyEvent` for event types).
    #[cfg(feature = "input")]
    fn use_key_listener_when(&mut self, predicate: bool, listener: impl Fn(VPlainContext2<'_, '_, Props, ViewData>, &KeyEvent) + 'static) {
        _use_key_listener_when(self, predicate, listener)
    }

    /// Register a function which will be called every time there is a mouse event (see `MouseEvent` for event types).
    #[cfg(feature = "input")]
    fn use_mouse_listener(&mut self, listener: impl Fn(VPlainContext2<'_, '_, Props, ViewData>, &MouseEvent) + 'static) {
        _use_mouse_listener(self, listener)
    }

    /// Register a function which will be called every time there is a mouse event (see `MouseEvent` for event types).
    #[cfg(feature = "input")]
    fn use_mouse_listener_when(&mut self, predicate: bool, listener: impl Fn(VPlainContext2<'_, '_, Props, ViewData>, &MouseEvent) + 'static) {
        _use_mouse_listener_when(self, predicate, listener)
    }


    /// Register a function which will be called every time there is a resize (window or column) event (see `ResizeEvent` for event types).
    #[cfg(feature = "input")]
    fn use_resize_listener(&mut self, listener: impl Fn(VPlainContext2<'_, '_, Props, ViewData>, &ResizeEvent) + 'static) {
        _use_resize_listener(self, listener)
    }

    /// Register a function which will be called every time there is a resize (window or column) event (see `ResizeEvent` for event types).
    #[cfg(feature = "input")]
    fn use_resize_listener_when(&mut self, predicate: bool, listener: impl Fn(VPlainContext2<'_, '_, Props, ViewData>, &ResizeEvent) + 'static) {
        _use_resize_listener_when(self, predicate, listener)
    }

    /// Register a function which will be called on ticks near the interval.
    /// Note that the function can't be called exactly on the interval because the ticks may not line up,
    /// but it will be called as soon as possible when or after the tick and will not drift.
    #[cfg(feature = "time")]
    fn use_interval(&mut self, interval: Duration, call_first: CallFirst, listener: impl Fn(VPlainContext2<'_, '_, Props, ViewData>) + 'static) {
        _use_interval(self, interval, call_first, listener)
    }

    /// Register a function which will be called once after the specified delay.
    /// Note that the function can't be called exactly on the interval because the ticks may not line up,
    /// but it will be called as soon as possible when or after the tick.
    #[cfg(feature = "time")]
    fn use_delay(&mut self, delay: Duration, listener: impl FnOnce(VPlainContext2<'_, '_, Props, ViewData>) + 'static) {
        _use_delay(self, delay, listener)
    }
}
