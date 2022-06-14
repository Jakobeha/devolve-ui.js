use std::rc::{Rc, Weak};
use std::cell::RefCell;
use tokio::time::{Interval, interval, MissedTickBehavior};
use std::sync::Arc;
use std::ops::Deref;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use crate::core::misc::notify_flag::NotifyFlag;
use crate::core::renderer::engine::RenderEngine;
use crate::core::renderer::render::VRenderLayer;
use crate::core::renderer::renderer::Renderer;

#[cfg(feature = "time")]
pub struct Running<Engine: RenderEngine + 'static> {
    renderer: Weak<Renderer<Engine>>,
    interval: RefCell<Interval>,
    pub(super) is_done: Arc<NotifyFlag>
}

#[cfg(feature = "time")]
pub struct RcRunning<Engine: RenderEngine + 'static>(pub Rc<Running<Engine>>);

pub trait WeakRunning<Engine: RenderEngine + 'static> {
    fn upgrade(&self) -> Option<RcRunning<Engine>>;
}

impl <Engine: RenderEngine + 'static> WeakRunning<Engine> for Weak<Running<Engine>> {
    fn upgrade(&self) -> Option<RcRunning<Engine>> {
        self.upgrade().map(RcRunning)
    }
}

#[cfg(feature = "time")]
impl <Engine: RenderEngine + 'static> RcRunning<Engine> where Engine::RenderLayer: VRenderLayer {
    pub(super) fn new(renderer: &Rc<Renderer<Engine>>) -> Self {
        // Update once at start if necessary; polling waits interval before updating
        renderer.update_and_rerender();

        Self(Rc::new(Running {
            renderer: Rc::downgrade(renderer),
            interval: RefCell::new(Self::mk_interval(renderer)),
            is_done: Arc::new(NotifyFlag::new())
        }))
    }

    fn tick(self: &Pin<&mut Self>) -> Poll<()> {
        let renderer = self.renderer.upgrade();
        if self.is_done.get() || renderer.is_none() {
            // Done
            return Poll::Ready(());
        }
        let renderer = renderer.unwrap();

        renderer.tick();
        renderer.update_and_rerender();

        // Not done (gets polled again and calls interval's poll)
        Poll::Pending
    }
}

#[cfg(feature = "time")]
impl <Engine: RenderEngine + 'static> RcRunning<Engine> {
    fn mk_interval(renderer: &Rc<Renderer<Engine>>) -> Interval {
        let mut interval = interval(renderer.interval_between_frames());
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        interval
    }

    #[allow(clippy::needless_lifetimes)] // Not needless for some reason
    pub fn is_done<'a>(&'a self) -> &'a Arc<NotifyFlag> {
        &self.is_done
    }

    pub(super) fn sync_interval(&self) {
        let renderer = self.renderer.upgrade();
        if renderer.is_none() {
            return;
        }
        let renderer = renderer.unwrap();
        *self.interval.borrow_mut() = Self::mk_interval(&renderer);
    }
}

impl <Engine: RenderEngine + 'static> Future for RcRunning<Engine> where Engine::RenderLayer: VRenderLayer {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.interval.borrow_mut().poll_tick(cx) {
            Poll::Ready(_) => self.tick(),
            Poll::Pending => Poll::Pending
        }
    }
}

impl <Engine: RenderEngine + 'static> Clone for RcRunning<Engine> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }

    fn clone_from(&mut self, source: &Self) {
        self.0.clone_from(&source.0);
    }
}

impl <Engine: RenderEngine + 'static> Deref for RcRunning<Engine> {
    type Target = Running<Engine>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
