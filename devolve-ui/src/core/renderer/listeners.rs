//! Listen for and emit events: time, keys, mouse, and resize.
//! So this is the observer pattern.

use std::marker::PhantomData;
#[cfg(feature = "time")]
use std::time::Duration;
#[cfg(feature = "input")]
use crate::core::renderer::input::{KeyEvent, MouseEvent, ResizeEvent};

pub(super) struct RendererListeners {
    #[cfg(feature = "time")]
    pub time: RendererListenersFor<Duration>,
    #[cfg(feature = "input")]
    pub keys: RendererListenersFor<KeyEvent>,
    #[cfg(feature = "input")]
    pub mouse: RendererListenersFor<MouseEvent>,
    #[cfg(feature = "input")]
    pub resize: RendererListenersFor<ResizeEvent>
}

pub(super) struct RendererListenersFor<Event>(Vec<Option<RendererListener<Event>>>);

pub type RendererListener<Event> = Box<dyn Fn(&Event)>;

pub struct RendererListenerId<Event> {
    index: usize,
    phantom: PhantomData<Event>
}

impl RendererListeners {
    pub(super) fn new() -> Self {
        RendererListeners {
            #[cfg(feature = "time")]
            time: RendererListenersFor::new(),
            #[cfg(feature = "input")]
            keys: RendererListenersFor::new(),
            #[cfg(feature = "input")]
            mouse: RendererListenersFor::new(),
            #[cfg(feature = "input")]
            resize: RendererListenersFor::new()
        }
    }
}

impl <Event> RendererListenersFor<Event> {
    pub(super) fn new() -> Self {
        Self(Vec::new())
    }

    pub(super) fn add(&mut self, listener: RendererListener<Event>) -> RendererListenerId<Event> {
        let index = self.0.len();
        self.0.push(Some(listener));
        RendererListenerId {
            index,
            phantom: PhantomData
        }
    }

    pub(super) fn remove(&mut self, listener_id: RendererListenerId<Event>) {
        assert!(listener_id.index < self.0.len());
        assert!(self.0[listener_id.index].is_some());
        self.0[listener_id.index] = None;
    }

    pub(super) fn run(&self, event: &Event) {
        for listener in self.0.iter() {
            if let Some(listener) = listener {
                listener(&event);
            }
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

