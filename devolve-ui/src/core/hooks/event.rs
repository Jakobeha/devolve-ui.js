//! Perform side-effects and modify state when you get events, AKA external input.
//! Events are:
//!
//! - Tick (passage of time. Only supported if you enable the `time` feature and call `Renderer::resume`)
//! - Key events
//! - Mouse events
//! - Resize events (resize window or change column size)
//!
//! Note that not all events are supported on all platforms. Supported events depend on the `RenderEngine` used by the renderer.

use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;
#[cfg(feature = "time")]
use std::time::{Duration, Instant};
use crate::core::component::context::{VComponentContext1, VContext, VDestructorContext2, VEffectContext2, VPlainContext2, with_plain_context};
use crate::core::component::root::VComponentRoot;
use crate::core::hooks::BuiltinHooks;
#[cfg(feature = "input")]
use crate::core::renderer::input::{KeyEvent, MouseEvent, ResizeEvent};
use crate::core::hooks::effect::{NoDependencies, UseEffectRerun};
use crate::core::hooks::state_internal::InternalHooks;
use crate::core::renderer::listeners::RendererListenerId;
use crate::core::view::view::VViewData;

fn _use_event_listener<'a, 'a0: 'a, Props: Any, Event: 'static, ViewData: VViewData + 'static>(
    c: &mut VComponentContext1<'a, 'a0, Props, ViewData>,
    rerun: UseEffectRerun<NoDependencies>,
    register_listener: impl Fn(VEffectContext2<'_, '_, Props, ViewData>, Rc<dyn VComponentRoot<ViewData=ViewData>>) -> RendererListenerId<Event> + 'static,
    unregister_listener: impl Fn(VDestructorContext2<'_, '_, Props, ViewData>, Rc<dyn VComponentRoot<ViewData=ViewData>>, RendererListenerId<Event>) + 'static
) {
    let unregister_listener = Rc::new(unregister_listener);
    c.use_effect(rerun, move |(c, props)| {
        let weak_renderer = c.component_imm().renderer();
        let renderer = weak_renderer.upgrade();

        if renderer.is_none() {
            log::warn!("can't use event on this component because it has no renderer");
        }

        let listener_id = match renderer {
            None => None,
            Some(renderer) => Some(register_listener((c, props), renderer))
        };

        let unregister_listener = unregister_listener.clone();
        move |(c, props)| {
            if let (Some(listener_id), Some(renderer)) = (listener_id, weak_renderer.upgrade()) {
                unregister_listener((c, props), renderer, listener_id);
            }
        }
    })
}

/// Register a function which will be called every time there is a tick event.
/// The duration is subject to delta-time diff, so use absolute intervals if you want precise time.
#[cfg(feature = "time")]
fn _use_tick_listener2<'a, 'a0: 'a, Props: Any, ViewData: VViewData + 'static>(
    c: &mut VComponentContext1<'a, 'a0, Props, ViewData>,
    rerun: UseEffectRerun<NoDependencies>,
    listener: impl Fn(VPlainContext2<'_, '_, Props, ViewData>, &Duration) + 'static
) {
    let listener = Rc::new(listener);
    _use_event_listener(c, rerun, move |(mut c, _props), renderer| {
        let c_ref = c.vref();
        let listener = listener.clone();
        renderer.clone().listen_for_time(Box::new(move |delta_time| {
            let listener = listener.clone();
            c_ref.try_with(move |(c, props)| {
                listener((c, props), &delta_time);
            });
        }))
    }, |_c, renderer, listener_id| {
        renderer.unlisten_for_time(listener_id)
    });
}

/// Register a function which will be called every time there is a tick event.
/// The duration is subject to delta-time diff, so use absolute intervals if you want precise time.
#[cfg(feature = "time")]
pub(super) fn _use_tick_listener<'a, 'a0: 'a, Props: Any, ViewData: VViewData + 'static>(
    c: &mut VComponentContext1<'a, 'a0, Props, ViewData>,
    listener: impl Fn(VPlainContext2<'_, '_, Props, ViewData>, &Duration) + 'static
) {
    _use_tick_listener2(c, UseEffectRerun::OnCreate, listener)
}

/// Register a function which will be called every time there is a tick event.
/// The duration is subject to delta-time diff, so use absolute intervals if you want precise time.
#[cfg(feature = "time")]
pub(super) fn _use_tick_listener_when<'a, 'a0: 'a, Props: Any, ViewData: VViewData + 'static>(
    c: &mut VComponentContext1<'a, 'a0, Props, ViewData>,
    predicate: bool,
    listener: impl Fn(VPlainContext2<'_, '_, Props, ViewData>, &Duration) + 'static
) {
    _use_tick_listener2(c, UseEffectRerun::OnPredicate(predicate), listener)
}

/// Register a function which will be called every time there is a key event (see `KeyEvent` for event types).
#[cfg(feature = "input")]
fn _use_key_listener2<'a, 'a0: 'a, Props: Any, ViewData: VViewData + 'static>(
    c: &mut VComponentContext1<'a, 'a0, Props, ViewData>,
    rerun: UseEffectRerun<NoDependencies>,
    listener: impl Fn(VPlainContext2<'_, '_, Props, ViewData>, &KeyEvent) + 'static
) {
    let listener = Rc::new(listener);
    _use_event_listener(c, rerun, move |(mut c, _props), renderer| {
        let c_ref = c.vref();
        let listener = listener.clone();
        renderer.listen_for_keys(Box::new(move |event| {
            let listener = listener.clone();
            c_ref.try_with(move |(c, props)| {
                listener((c, props), &event);
            });
        }))
    }, |_c, renderer, listener_id| {
        renderer.unlisten_for_keys(listener_id)
    });
}

/// Register a function which will be called every time there is a key event (see `KeyEvent` for event types).
#[cfg(feature = "input")]
pub(super) fn _use_key_listener<'a, 'a0: 'a, Props: Any, ViewData: VViewData + 'static>(
    c: &mut VComponentContext1<'a, 'a0, Props, ViewData>,
    listener: impl Fn(VPlainContext2<'_, '_, Props, ViewData>, &KeyEvent) + 'static
) {
    _use_key_listener2(c, UseEffectRerun::OnCreate, listener)
}

/// Register a function which will be called every time there is a key event (see `KeyEvent` for event types).
#[cfg(feature = "input")]
pub(super) fn _use_key_listener_when<'a, 'a0: 'a, Props: Any, ViewData: VViewData + 'static>(
    c: &mut VComponentContext1<'a, 'a0, Props, ViewData>,
    predicate: bool,
    listener: impl Fn(VPlainContext2<'_, '_, Props, ViewData>, &KeyEvent) + 'static
) {
    _use_key_listener2(c, UseEffectRerun::OnPredicate(predicate), listener)
}

/// Register a function which will be called every time there is a mouse event (see `MouseEvent` for event types).
#[cfg(feature = "input")]
fn _use_mouse_listener2<'a, 'a0: 'a, Props: Any, ViewData: VViewData + 'static>(
    c: &mut VComponentContext1<'a, 'a0, Props, ViewData>,
    rerun: UseEffectRerun<NoDependencies>,
    listener: impl Fn(VPlainContext2<'_, '_, Props, ViewData>, &MouseEvent) + 'static
) {
    let listener = Rc::new(listener);
    _use_event_listener(c, rerun,move |(mut c, _props), renderer| {
        let c_ref = c.vref();
        let listener = listener.clone();
        renderer.listen_for_mouse(Box::new(move |event| {
            let listener = listener.clone();
            c_ref.try_with(move |(c, props)| {
                listener((c, props), &event);
            });
        }))
    }, |_c, renderer, listener_id| {
        renderer.unlisten_for_mouse(listener_id)
    });
}

/// Register a function which will be called every time there is a mouse event (see `MouseEvent` for event types).
#[cfg(feature = "input")]
pub(super) fn _use_mouse_listener<'a, 'a0: 'a, Props: Any, ViewData: VViewData + 'static>(
    c: &mut VComponentContext1<'a, 'a0, Props, ViewData>,
    listener: impl Fn(VPlainContext2<'_, '_, Props, ViewData>, &MouseEvent) + 'static
) {
    _use_mouse_listener2(c, UseEffectRerun::OnCreate, listener)
}

/// Register a function which will be called every time there is a mouse event (see `MouseEvent` for event types).
#[cfg(feature = "input")]
pub(super) fn _use_mouse_listener_when<'a, 'a0: 'a, Props: Any, ViewData: VViewData + 'static>(
    c: &mut VComponentContext1<'a, 'a0, Props, ViewData>,
    predicate: bool,
    listener: impl Fn(VPlainContext2<'_, '_, Props, ViewData>, &MouseEvent) + 'static
) {
    _use_mouse_listener2(c, UseEffectRerun::OnPredicate(predicate), listener)
}

/// Register a function which will be called every time there is a resize (window or column) event (see `ResizeEvent` for event types).
#[cfg(feature = "input")]
fn _use_resize_listener2<'a, 'a0: 'a, Props: Any, ViewData: VViewData + 'static>(
    c: &mut VComponentContext1<'a, 'a0, Props, ViewData>,
    rerun: UseEffectRerun<NoDependencies>,
    listener: impl Fn(VPlainContext2<'_, '_, Props, ViewData>, &ResizeEvent) + 'static
) {
    let listener = Rc::new(listener);
    _use_event_listener(c, rerun, move |(mut c, _props), renderer| {
        let c_ref = c.vref();
        let listener = listener.clone();
        renderer.listen_for_resize(Box::new(move |event| {
            let listener = listener.clone();
            c_ref.try_with(move |(c, props)| {
                listener((c, props), &event);
            });
        }))
    }, |_c, renderer, listener_id| {
        renderer.unlisten_for_resize(listener_id)
    });
}


/// Register a function which will be called every time there is a resize (window or column) event (see `ResizeEvent` for event types).
#[cfg(feature = "input")]
pub(super) fn _use_resize_listener<'a, 'a0: 'a, Props: Any, ViewData: VViewData + 'static>(
    c: &mut VComponentContext1<'a, 'a0, Props, ViewData>,
    listener: impl Fn(VPlainContext2<'_, '_, Props, ViewData>, &ResizeEvent) + 'static
) {
    _use_resize_listener2(c, UseEffectRerun::OnCreate, listener)
}

/// Register a function which will be called every time there is a resize (window or column) event (see `ResizeEvent` for event types).
#[cfg(feature = "input")]
pub(super) fn _use_resize_listener_when<'a, 'a0: 'a, Props: Any, ViewData: VViewData + 'static>(
    c: &mut VComponentContext1<'a, 'a0, Props, ViewData>,
    predicate: bool,
    listener: impl Fn(VPlainContext2<'_, '_, Props, ViewData>, &ResizeEvent) + 'static
) {
    _use_resize_listener2(c, UseEffectRerun::OnPredicate(predicate), listener)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallFirst {
    Immediately,
    AfterTheInterval
}

/// Register a function which will be called on ticks near the interval.
/// Note that the function can't be called exactly on the interval because the ticks may not line up,
/// but it will be called as soon as possible when or after the tick and will not drift.
#[cfg(feature = "time")]
pub(super) fn _use_interval<'a, 'a0: 'a, Props: Any, ViewData: VViewData + 'static>(
    c: &mut VComponentContext1<'a, 'a0, Props, ViewData>,
    interval: Duration,
    call_first: CallFirst,
    listener: impl Fn(VPlainContext2<'_, '_, Props, ViewData>) + 'static
) {
    let listener = Rc::new(listener);
    let listener2 = listener.clone();
    if call_first == CallFirst::Immediately {
        c.use_effect(UseEffectRerun::OnCreate, move |(mut c, props)| {
            with_plain_context((&mut c, props), |(c, props)| listener((c, props)));
            return |(_c, _props)| {};
        });
    }
    let listener = listener2;

    let last_call = Rc::new(RefCell::new(Instant::now()));
    c.use_tick_listener(move |(mut c, props), _delta_time| {
        let mut last_call = last_call.borrow_mut();
        let mut elapsed = last_call.elapsed();
        while elapsed >= interval {
            *last_call += interval;
            elapsed -= interval;
            // Don't expect this to get last_call borrowed again
            c.with(|c| listener((c, props)));
        }
    });
}

/// Register a function which will be called once after the specified delay.
/// Note that the function can't be called exactly on the interval because the ticks may not line up,
/// but it will be called as soon as possible when or after the tick.
#[cfg(feature = "time")]
pub(super) fn _use_delay<'a, 'a0: 'a, Props: Any, ViewData: VViewData + 'static>(
    c: &mut VComponentContext1<'a, 'a0, Props, ViewData>,
    delay: Duration,
    listener: impl FnOnce(VPlainContext2<'_, '_, Props, ViewData>) + 'static
) {
    let listener = RefCell::new(Some(listener));
    let called = c.use_non_updating_state(|_| false);
    let start_time = Instant::now();
    let predicate = !*called.get(c);
    _use_tick_listener_when(c, !predicate, move |c, _delta_time| {
        let elapsed = start_time.elapsed();
        if elapsed >= delay {
            let listener = listener.borrow_mut().take().expect("broken invariant: somehow called the delayed listener twice");
            listener(c);
        }
    });
}