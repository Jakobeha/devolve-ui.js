use std::rc::Rc;
use crate::core::component::component::VComponent;
#[cfg(feature = "time")]
use std::time::Duration;
use crate::core::component::root::VComponentRoot;
#[cfg(feature = "input")]
use crate::core::misc::input::{KeyEvent, MouseEvent, ResizeEvent};
use crate::core::hooks::effect::{use_effect, UseEffectRerun};
use crate::core::renderer::listeners::RendererListenerId;
use crate::core::view::view::VViewData;

fn use_event_listener<Event: 'static, ViewData: VViewData + 'static>(
    c: &mut Box<VComponent<ViewData>>,
    register_listener: impl Fn(&mut Box<VComponent<ViewData>>, Rc<dyn VComponentRoot<ViewData = ViewData>>) -> RendererListenerId<Event> + 'static,
    unregister_listener: impl Fn(&mut Box<VComponent<ViewData>>, Rc<dyn VComponentRoot<ViewData = ViewData>>, RendererListenerId<Event>) + 'static
) {
    let unregister_listener = Rc::new(unregister_listener);
    use_effect(c, UseEffectRerun::OnCreate, move |c| {
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
pub fn use_tick_listener<ViewData: VViewData + 'static>(
    c: &mut Box<VComponent<ViewData>>,
    listener: impl Fn(&mut Box<VComponent<ViewData>>, &Duration) + 'static
) {
    let listener = Rc::new(listener);
    use_event_listener(c, move |c, renderer| {
        let c_path = c.path();
        let renderer2 = renderer.clone();
        let listener = listener.clone();
        renderer.listen_for_time(Box::new(move |delta_time| {
            let listener = listener.clone();
            // We have to clone because the type is Rc<Self>, not &Rc<Self>, since the latter isn't object-safe.
            // Why isn't the latter object-safe? :(
            renderer2.clone().with_component(&c_path, Box::new(move |c| {
                if let Some(c) = c {
                    listener(c, &delta_time);
                }
            }));
        }))
    }, |_c, renderer, listener_id| {
        renderer.unlisten_for_time(listener_id)
    });
}

/// Register a function which will be called every time there is a key event (see `KeyEvent` for event types).
#[cfg(feature = "input")]
pub fn use_key_listener<ViewData: VViewData + 'static>(
    c: &mut Box<VComponent<ViewData>>,
    listener: impl Fn(&mut Box<VComponent<ViewData>>, &KeyEvent) + 'static
) {
    let listener = Rc::new(listener);
    use_event_listener(c, move |c, renderer| {
        let c_path = c.path();
        let renderer2 = renderer.clone();
        let listener = listener.clone();
        renderer.listen_for_keys(Box::new(move |event| {
            let listener = listener.clone();
            // We have to clone because the type is Rc<Self>, not &Rc<Self>, since the latter isn't object-safe.
            // Why isn't the latter object-safe? :(
            renderer2.clone().with_component(&c_path, Box::new(move |c| {
                if let Some(c) = c {
                    listener(c, &event);
                }
            }));
        }))
    }, |_c, renderer, listener_id| {
        renderer.unlisten_for_keys(listener_id)
    });
}

/// Register a function which will be called every time there is a mouse event (see `MouseEvent` for event types).
#[cfg(feature = "input")]
pub fn use_mouse_listener<ViewData: VViewData + 'static>(
    c: &mut Box<VComponent<ViewData>>,
    listener: impl Fn(&mut Box<VComponent<ViewData>>, &MouseEvent) + 'static
) {
    let listener = Rc::new(listener);
    use_event_listener(c, move |c, renderer| {
        let c_path = c.path();
        let renderer2 = renderer.clone();
        let listener = listener.clone();
        renderer.listen_for_mouse(Box::new(move |event| {
            let listener = listener.clone();
            // We have to clone because the type is Rc<Self>, not &Rc<Self>, since the latter isn't object-safe.
            // Why isn't the latter object-safe? :(
            renderer2.clone().with_component(&c_path, Box::new(move |c| {
                if let Some(c) = c {
                    listener(c, &event);
                }
            }));
        }))
    }, |_c, renderer, listener_id| {
        renderer.unlisten_for_mouse(listener_id)
    });
}

/// Register a function which will be called every time there is a resize (window or column) event (see `ResizeEvent` for event types).
#[cfg(feature = "input")]
pub fn use_resize_listener<ViewData: VViewData + 'static>(
    c: &mut Box<VComponent<ViewData>>,
    listener: impl Fn(&mut Box<VComponent<ViewData>>, &ResizeEvent) + 'static
) {
    let listener = Rc::new(listener);
    use_event_listener(c, move |c, renderer| {
        let c_path = c.path();
        let renderer2 = renderer.clone();
        let listener = listener.clone();
        renderer.listen_for_resize(Box::new(move |event| {
            let listener = listener.clone();
            // We have to clone because the type is Rc<Self>, not &Rc<Self>, since the latter isn't object-safe.
            // Why isn't the latter object-safe? :(
            renderer2.clone().with_component(&c_path, Box::new(move |c| {
                if let Some(c) = c {
                    listener(c, &event);
                }
            }));
        }))
    }, |_c, renderer, listener_id| {
        renderer.unlisten_for_resize(listener_id)
    });
}
