use std::cell::RefCell;
use std::rc::Rc;
use crate::core::component::component::VComponent;
#[cfg(feature = "time")]
use std::time::{Duration, Instant};
use crate::core::component::root::VComponentRoot;
#[cfg(feature = "input")]
use crate::core::misc::input::{KeyEvent, MouseEvent, ResizeEvent};
use crate::core::hooks::effect::{NoDependencies, UseEffectRerun, use_effect};
use crate::core::hooks::state_internal::use_non_updating_state;
use crate::core::renderer::listeners::RendererListenerId;
use crate::core::view::view::VViewData;

fn _use_event_listener<Event: 'static, ViewData: VViewData + 'static>(
    c: &mut Box<VComponent<ViewData>>,
    rerun: UseEffectRerun<NoDependencies>,
    register_listener: impl Fn(&mut Box<VComponent<ViewData>>, Rc<dyn VComponentRoot<ViewData = ViewData>>) -> RendererListenerId<Event> + 'static,
    unregister_listener: impl Fn(&mut Box<VComponent<ViewData>>, Rc<dyn VComponentRoot<ViewData = ViewData>>, RendererListenerId<Event>) + 'static
) {
    let unregister_listener = Rc::new(unregister_listener);
    use_effect(c, rerun, move |c| {
        let weak_renderer = c.renderer();
        let renderer = weak_renderer.upgrade();

        if renderer.is_none() {
            eprintln!("can't use event on this component because it has no renderer");
        }

        let listener_id = renderer.map(|renderer| register_listener(c, renderer));

        let unregister_listener = unregister_listener.clone();
        move |c| {
            if let (Some(listener_id), Some(renderer)) = (listener_id, weak_renderer.upgrade()) {
                unregister_listener(c, renderer, listener_id);
            }
        }
    })
}

/// Register a function which will be called every time there is a tick event.
/// The duration is subject to delta-time diff, so use absolute intervals if you want precise time.
#[cfg(feature = "time")]
fn _use_tick_listener<ViewData: VViewData + 'static>(
    c: &mut Box<VComponent<ViewData>>,
    rerun: UseEffectRerun<NoDependencies>,
    listener: impl Fn(&mut Box<VComponent<ViewData>>, &Duration) + 'static
) {
    let listener = Rc::new(listener);
    _use_event_listener(c, rerun, move |c, renderer| {
        let c_ref = c.vref();
        let listener = listener.clone();
        renderer.clone().listen_for_time(Box::new(move |delta_time| {
            let listener = listener.clone();
            c_ref.try_with(move |c| {
                listener(c, &delta_time);
            });
        }))
    }, |_c, renderer, listener_id| {
        renderer.unlisten_for_time(listener_id)
    });
}

/// Register a function which will be called every time there is a tick event.
/// The duration is subject to delta-time diff, so use absolute intervals if you want precise time.
#[cfg(feature = "time")]
pub fn use_tick_listener<ViewData: VViewData + 'static>(
    c: &mut Box<VComponent<ViewData>>,
    listener: impl Fn(&mut Box<VComponent<ViewData>>, &Duration) + 'static
) {
    _use_tick_listener(c, UseEffectRerun::OnCreate, listener)
}

/// Register a function which will be called every time there is a tick event.
/// The duration is subject to delta-time diff, so use absolute intervals if you want precise time.
#[cfg(feature = "time")]
pub fn use_tick_listener_when<ViewData: VViewData + 'static>(
    c: &mut Box<VComponent<ViewData>>,
    predicate: bool,
    listener: impl Fn(&mut Box<VComponent<ViewData>>, &Duration) + 'static
) {
    _use_tick_listener(c, UseEffectRerun::OnPredicate(predicate), listener)
}

/// Register a function which will be called every time there is a key event (see `KeyEvent` for event types).
#[cfg(feature = "input")]
fn _use_key_listener<ViewData: VViewData + 'static>(
    c: &mut Box<VComponent<ViewData>>,
    rerun: UseEffectRerun<NoDependencies>,
    listener: impl Fn(&mut Box<VComponent<ViewData>>, &KeyEvent) + 'static
) {
    let listener = Rc::new(listener);
    _use_event_listener(c, rerun, move |c, renderer| {
        let c_ref = c.vref();
        let listener = listener.clone();
        renderer.listen_for_keys(Box::new(move |event| {
            let listener = listener.clone();
            c_ref.try_with(move |c| {
                listener(c, &event);
            });
        }))
    }, |_c, renderer, listener_id| {
        renderer.unlisten_for_keys(listener_id)
    });
}

/// Register a function which will be called every time there is a key event (see `KeyEvent` for event types).
#[cfg(feature = "input")]
pub fn use_key_listener<ViewData: VViewData + 'static>(
    c: &mut Box<VComponent<ViewData>>,
    listener: impl Fn(&mut Box<VComponent<ViewData>>, &KeyEvent) + 'static
) {
    _use_key_listener(c, UseEffectRerun::OnCreate, listener)
}

/// Register a function which will be called every time there is a key event (see `KeyEvent` for event types).
#[cfg(feature = "input")]
pub fn use_key_listener_when<ViewData: VViewData + 'static>(
    c: &mut Box<VComponent<ViewData>>,
    predicate: bool,
    listener: impl Fn(&mut Box<VComponent<ViewData>>, &KeyEvent) + 'static
) {
    _use_key_listener(c, UseEffectRerun::OnPredicate(predicate), listener)
}

/// Register a function which will be called every time there is a mouse event (see `MouseEvent` for event types).
#[cfg(feature = "input")]
fn _use_mouse_listener<ViewData: VViewData + 'static>(
    c: &mut Box<VComponent<ViewData>>,
    rerun: UseEffectRerun<NoDependencies>,
    listener: impl Fn(&mut Box<VComponent<ViewData>>, &MouseEvent) + 'static
) {
    let listener = Rc::new(listener);
    _use_event_listener(c, rerun,move |c, renderer| {
        let c_ref = c.vref();
        let listener = listener.clone();
        renderer.listen_for_mouse(Box::new(move |event| {
            let listener = listener.clone();
            c_ref.try_with(move |c| {
                listener(c, &event);
            });
        }))
    }, |_c, renderer, listener_id| {
        renderer.unlisten_for_mouse(listener_id)
    });
}

/// Register a function which will be called every time there is a mouse event (see `MouseEvent` for event types).
#[cfg(feature = "input")]
pub fn use_mouse_listener<ViewData: VViewData + 'static>(
    c: &mut Box<VComponent<ViewData>>,
    listener: impl Fn(&mut Box<VComponent<ViewData>>, &MouseEvent) + 'static
) {
    _use_mouse_listener(c, UseEffectRerun::OnCreate, listener)
}

/// Register a function which will be called every time there is a mouse event (see `MouseEvent` for event types).
#[cfg(feature = "input")]
pub fn use_mouse_listener_when<ViewData: VViewData + 'static>(
    c: &mut Box<VComponent<ViewData>>,
    predicate: bool,
    listener: impl Fn(&mut Box<VComponent<ViewData>>, &MouseEvent) + 'static
) {
    _use_mouse_listener(c, UseEffectRerun::OnPredicate(predicate), listener)
}

/// Register a function which will be called every time there is a resize (window or column) event (see `ResizeEvent` for event types).
#[cfg(feature = "input")]
fn _use_resize_listener<ViewData: VViewData + 'static>(
    c: &mut Box<VComponent<ViewData>>,
    rerun: UseEffectRerun<NoDependencies>,
    listener: impl Fn(&mut Box<VComponent<ViewData>>, &ResizeEvent) + 'static
) {
    let listener = Rc::new(listener);
    _use_event_listener(c, rerun, move |c, renderer| {
        let c_ref = c.vref();
        let listener = listener.clone();
        renderer.listen_for_resize(Box::new(move |event| {
            let listener = listener.clone();
            c_ref.try_with(move |c| {
                listener(c, &event);
            });
        }))
    }, |_c, renderer, listener_id| {
        renderer.unlisten_for_resize(listener_id)
    });
}


/// Register a function which will be called every time there is a resize (window or column) event (see `ResizeEvent` for event types).
#[cfg(feature = "input")]
pub fn use_resize_listener<ViewData: VViewData + 'static>(
    c: &mut Box<VComponent<ViewData>>,
    listener: impl Fn(&mut Box<VComponent<ViewData>>, &ResizeEvent) + 'static
) {
    _use_resize_listener(c, UseEffectRerun::OnCreate, listener)
}

/// Register a function which will be called every time there is a resize (window or column) event (see `ResizeEvent` for event types).
#[cfg(feature = "input")]
pub fn use_resize_listener_when<ViewData: VViewData + 'static>(
    c: &mut Box<VComponent<ViewData>>,
    predicate: bool,
    listener: impl Fn(&mut Box<VComponent<ViewData>>, &ResizeEvent) + 'static
) {
    _use_resize_listener(c, UseEffectRerun::OnPredicate(predicate), listener)
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
pub fn use_interval<ViewData: VViewData + 'static>(
    c: &mut Box<VComponent<ViewData>>,
    interval: Duration,
    call_first: CallFirst,
    listener: impl Fn(&mut Box<VComponent<ViewData>>) + 'static
) {
    let listener = Rc::new(listener);
    let listener2 = listener.clone();
    if call_first == CallFirst::Immediately {
        use_effect(c, UseEffectRerun::OnCreate, move |c| {
            listener(c);
            return |_c| {};
        });
    }
    let listener = listener2;

    let last_call = Rc::new(RefCell::new(Instant::now()));
    use_tick_listener(c, move |c, _delta_time| {
        let mut last_call = last_call.borrow_mut();
        let mut elapsed = last_call.elapsed();
        while elapsed >= interval {
            *last_call += interval;
            elapsed -= interval;
            // Don't expect this to get last_call borrowed again
            listener(c);
        }
    });
}

/// Register a function which will be called once after the specified delay.
/// Note that the function can't be called exactly on the interval because the ticks may not line up,
/// but it will be called as soon as possible when or after the tick.
#[cfg(feature = "time")]
pub fn use_delay<ViewData: VViewData + 'static>(
    c: &mut Box<VComponent<ViewData>>,
    delay: Duration,
    listener: impl FnOnce(&mut Box<VComponent<ViewData>>) + 'static
) {
    let listener = RefCell::new(Some(listener));
    let called = use_non_updating_state(c, || false);
    let start_time = Instant::now();
    use_tick_listener_when(c, !*called.get(c), move |c, _delta_time| {
        let elapsed = start_time.elapsed();
        if elapsed >= delay {
            let listener = listener.borrow_mut().take().expect("broken invariant: somehow called the delayed listener twice");
            listener(c);
        }
    });
}