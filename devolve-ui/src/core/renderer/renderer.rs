use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
#[cfg(feature = "time")]
use std::time::Duration;
#[cfg(feature = "time")]
use tokio::time::{interval, Interval};
#[cfg(feature = "time")]
use tokio::task::{spawn, JoinHandle};
use crate::core::component::component::VComponent;
use crate::core::component::context::VContext;
use crate::core::component::node::{NodeId, VNode};
use crate::core::view::layout::geom::Rectangle;
use crate::core::view::layout::parent_bounds::ParentBounds;
use crate::core::view::view::VView;
use crate::core::renderer::render::VRender;
use crate::core::renderer::engine::RenderEngine;

struct CachedRender<Layer> {
    render: VRender<Layer>,
    parent_bounds: ParentBounds,
    sibling_rect: Option<Rectangle>,
    parent: NodeId
}

type RunningTask = JoinHandle<()>;

pub struct Renderer<Engine: RenderEngine> {
    engine: RefCell<Engine>,
    is_visible: Cell<bool>,
    #[cfg(feature = "time")]
    interval_between_frames: Cell<Duration>,
    #[cfg(feature = "time")]
    running_task: RefCell<Option<RunningTask>>,
    cached_renders: RefCell<HashMap<NodeId, CachedRender<Engine::RenderLayer>>>,
    needs_rerender: Cell<bool>,
    root_component: RefCell<Option<Box<VComponent<ViewData>>>>
}

impl <Engine: RenderEngine> Renderer<Engine> {
    pub const DEFAULT_INTERVAL_BETWEEN_FRAMES: Duration = Duration::from_millis(25);

    /// Creates a new renderer.
    ///
    /// # Examples
    /// Start a renderer which re-renders automatically
    /// ```
    /// use devolve_ui::core::renderer::renderer::Renderer;
    /// use devolve_ui::rsx;
    /// use std::time::Duration;
    ///
    /// let renderer = Renderer::new(TODOEngine);
    /// renderer.root(rsx! { ... });
    /// renderer.interval_between_frames(Duration::from_millis(25)); // optional
    /// renderer.show();
    /// renderer.resume();
    /// ```
    ///
    /// Start a renderer which re-renders manually (renders once, then can be re-rendered via `renderer.rerender()`)
    /// ```
    /// use devolve_ui::core::renderer::renderer::Renderer;
    /// use devolve_ui::rsx;
    /// use std::time::Duration;
    ///
    /// let renderer = Renderer::new(TODOEngine);
    /// renderer.root(rsx! { ... });
    /// renderer.show();
    /// ```
    pub fn new(engine: Engine) -> Rc<Self> {
        let renderer = Rc::new(Renderer {
            engine: RefCell::new(engine),
            is_visible: Cell::new(false),
            #[cfg(feature = "time")]
            interval_between_frames: Cell::new(Self::DEFAULT_INTERVAL_BETWEEN_FRAMES),
            #[cfg(feature = "time")]
            running_task: RefCell::new(None),
            cached_renders: RefCell::new(HashMap::new()),
            needs_rerender: Cell::new(false),
            root_component: RefCell::new(None)
        });
        renderer.engine.borrow().on_resize(Box::new(|| {
            renderer.needs_rerender.set(true);
        }));
        renderer
    }

    pub fn root(self: Rc<Self>, construct: impl FnOnce() -> Box<VComponent<ViewData>>) {
        VContext::with_local_context(|| {
            let root_component = VContext::with_push_renderer(Rc::downgrade(&self), construct);
            self.set_root_component(Some(root_component));
        });
        if self.is_visible {
            self.rerender();
        }
    }


    fn set_root_component(self: Rc<Self>, root_component: Option<Box<VComponent<ViewData>>>) {
        let mut self_root_component = self.root_component.borrow_mut();
        *self_root_component = root_component;
        if let Some(self_root_component) = self_root_component.as_mut() {
            self_root_component.update(Cow::Borrowed("init:"));
        }
    }

    #[cfg(feature = "time")]
    pub fn resume(self: Rc<Self>) {
        assert!(!self.is_running(), "already running");
        assert!(self.is_visible.get(), "can't resume render while invisible");

        *self.running_task.borrow_mut() = Some(spawn(async {
            let mut interval = interval(self.interval_between_frames.get());

            loop {
                if self.needs_rerender.get() && self.is_visible.get() {
                    self.rerender();
                }
                interval.tick().await;
            }
        }));
    }

    #[cfg(feature = "time")]
    pub fn pause(self: Rc<Self>) {
        assert!(self.is_running(), "not running");

        self.running_task.take().unwrap().abort();
    }

    #[cfg(feature = "time")]
    pub fn is_running(self: Rc<Self>) -> bool {
        self.running_task.borrow().is_some()
    }

    #[cfg(feature = "time")]
    pub fn set_interval(self: Rc<Self>, interval_between_frames: Duration) {
        // Need to pause and resume if running to change the interval
        let is_running = self.is_running();
        if is_running {
            self.pause();
        }
        self.interval_between_frames.set(interval_between_frames);
        if is_running {
            self.resume()
        }
    }

    /// Make the renderer visible and render once
    pub fn show(self: Rc<Self>) {
        self.is_visible.set(true);
        self.render(true);
    }

    /// Clear render and make the renderer invisible
    pub fn hide(self: Rc<Self>) {
        self.clear(true);
        self.is_visible.set(false);
    }

    /// Clears and rerenders immediately
    pub fn rerender(self: Rc<Self>) {
        assert!(self.is_visible.get(), "can't rerender while invisible");
        assert!(self.root_component.borrow().is_some(), "can't rerender without root component");

        self.clear(false);
        self.render(false);
    }

    fn render(self: Rc<Self>, is_first: bool) {
        assert!(self.is_visible.get(), "can't render while invisible");
        assert!(self.root_component.borrow().is_some(), "can't render without root component");

        self.needs_rerender.set(false);

        let mut engine = self.engine.borrow_mut();
        if is_first {
            engine.start_rendering();
        }
        // TODO
    }

    fn clear(self: Rc<Self>, is_last: bool) {
        // These assertions can't actually happen
        assert!(self.is_visible.get(), "can't clear render while invisible");
        assert!(self.root_component.borrow().is_some(), "can't clear render without root component");

        let mut engine = self.engine.borrow_mut();
        engine.clear();
        if is_last {
            engine.stop_rendering();
        }
    }

    pub(crate) fn invalidate(self: Rc<Self>, view: &Box<VView<ViewData>>) {
        // Removes this view and all parents from cached_renders
        let mut cached_renders = self.cached_renders.borrow_mut();
        let mut next_view_id = view.id();
        while next_view_id != VNode::NULL_ID {
            // This code 1) removes next_view_id from cached_renders,
            // 2) sets next_view_id to the parent (from cached_renders[next_view_id]),
            // and 3) if next_view_id wasn't actually in cached_renders, sets next_view_id to NULL_ID
            // so that the loop breaks.
            next_view_id = cached_renders.remove(&next_view_id)
                .map(|cached_render| cached_render.parent)
                .unwrap_or(VNode::NULL_ID);
        }

        self.needs_rerender.set(true);
    }
}

impl <Engine: RenderEngine> Drop for Renderer<Engine> {
    fn drop(&mut self) {
        if self.is_visible.get() {
            self.engine.borrow_mut().stop_rendering();
        }
    }
}