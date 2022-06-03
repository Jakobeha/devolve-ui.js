use std::borrow::Cow;
use std::cell::{Cell, RefCell, RefMut};
use std::collections::HashMap;
use std::future::Future;
use std::ops::Deref;
use std::pin::Pin;
use std::rc::{Rc, Weak};
use std::sync::{Arc, Weak as WeakArc};
use std::task::{Context, Poll};
#[cfg(feature = "time")]
use std::time::Duration;
#[cfg(feature = "time")]
use tokio::time::{Interval, interval, MissedTickBehavior};
#[cfg(feature = "time-blocking")]
use tokio::runtime;
use crate::core::component::component::{VComponent, VComponentBody, VComponentRoot};
use crate::core::component::node::{NodeId, VNode};
use crate::core::component::parent::{_VParent, VParent};
use crate::core::misc::notify_bool::FlagForOtherThreads;
use crate::core::view::layout::geom::Rectangle;
use crate::core::view::layout::parent_bounds::ParentBounds;
use crate::core::view::view::{VView, VViewData};
use crate::core::renderer::render::{VRender, VRenderLayer};
use crate::core::renderer::engine::RenderEngine;

struct CachedRender<Layer> {
    render: VRender<Layer>,
    parent_bounds: ParentBounds,
    prev_sibling: Option<Rectangle>,
    parent: NodeId
}

#[cfg(feature = "time")]
pub struct Running<Engine: RenderEngine + 'static> {
    renderer: Weak<Renderer<Engine>>,
    interval: RefCell<Interval>,
    is_done: Arc<FlagForOtherThreads>
}

#[cfg(feature = "time")]
pub struct RcRunning<Engine: RenderEngine + 'static>(pub Rc<Running<Engine>>);

#[cfg(feature = "time")]
impl <Engine: RenderEngine + 'static> RcRunning<Engine> where Engine::RenderLayer: VRenderLayer {
    fn new(renderer: &Rc<Renderer<Engine>>) -> Self {
        // Render once at start if necessary; polling waits interval before re-rendering
        if renderer.needs_rerender.get() && renderer.is_visible.get() {
            renderer.rerender();
        }

        Self(Rc::new(Running {
            renderer: Rc::downgrade(renderer),
            interval: RefCell::new(Self::mk_interval(renderer)),
            is_done: Arc::new(FlagForOtherThreads::new())
        }))
    }

    fn tick(self: &Pin<&mut Self>) -> Poll<()> {
        let renderer = self.renderer.upgrade();
        if self.is_done.get() || renderer.is_none() {
            // Done
            return Poll::Ready(());
        }
        let renderer = renderer.unwrap();

        if renderer.needs_rerender.get() && renderer.is_visible.get() {
            renderer.rerender();
        }

        // Not done (gets polled again and calls interval's poll)
        Poll::Pending
    }
}

#[cfg(feature = "time")]
impl <Engine: RenderEngine + 'static> RcRunning<Engine> {
    fn mk_interval(renderer: &Rc<Renderer<Engine>>) -> Interval {
        let mut interval = interval(renderer.interval_between_frames.get());
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        interval
    }

    #[allow(clippy::needless_lifetimes)] // Not needless for some reason
    pub fn is_done<'a>(&'a self) -> &'a Arc<FlagForOtherThreads> {
        &self.is_done
    }

    fn sync_interval(&self) {
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

pub trait WeakRunning<Engine: RenderEngine + 'static> {
    fn upgrade(&self) -> Option<RcRunning<Engine>>;
}

impl <Engine: RenderEngine + 'static> WeakRunning<Engine> for Weak<Running<Engine>> {
    fn upgrade(&self) -> Option<RcRunning<Engine>> {
        self.upgrade().map(RcRunning)
    }
}

pub struct Renderer<Engine: RenderEngine + 'static> {
    engine: RefCell<Engine>,
    is_visible: Cell<bool>,
    #[cfg(feature = "time")]
    interval_between_frames: Cell<Duration>,
    #[cfg(feature = "time")]
    running: RefCell<Option<RcRunning<Engine>>>,
    cached_renders: RefCell<HashMap<NodeId, CachedRender<Engine::RenderLayer>>>,
    needs_rerender: Arc<FlagForOtherThreads>,
    root_component: RefCell<Option<Box<VComponent<Engine::ViewData>>>>
}

struct RenderBorrows<'a, Engine: RenderEngine> {
    pub cached_renders: RefMut<'a, HashMap<NodeId, CachedRender<Engine::RenderLayer>>>,
    pub engine: RefMut<'a, Engine>
}

impl <Engine: RenderEngine> Renderer<Engine> where Engine::RenderLayer: VRenderLayer {
    #[cfg(feature = "time")]
    pub const DEFAULT_INTERVAL_BETWEEN_FRAMES: Duration = Duration::from_millis(25);

    /// Creates a new renderer.
    ///
    /// # Examples
    /// Start a renderer which re-renders automatically.
    /// The renderer will fully block the current thread until your app exits.
    /// ```
    /// use devolve_ui::core::renderer::renderer::Renderer;
    /// use devolve_ui::rsx;
    /// use std::time::Duration;
    ///
    /// let renderer = Renderer::new(TODOEngine);
    /// renderer.root(rsx! { ... });
    /// renderer.interval_between_frames(Duration::from_millis(25)); // optional
    /// renderer.show();
    /// renderer.resume_blocking();
    /// ```
    ///
    /// Start a renderer which re-renders automatically in an async block.
    /// The renderer will `await` in the current future but not block other concurrent futures.
    /// ```
    /// use devolve_ui::core::renderer::renderer::Renderer;
    /// use devolve_ui::rsx;
    /// use std::time::Duration;
    ///
    /// let renderer = Renderer::new(TODOEngine);
    /// renderer.root(rsx! { ... });
    /// renderer.interval_between_frames(Duration::from_millis(25)); // optional
    /// renderer.show();
    /// renderer.resume().await;
    /// ```
    ///
    /// Start a renderer which re-renders on a background thread.
    /// You can stop the renderer by closing the thread, or calling `escape.upgrade.expect("renderer was already stopped").set()`.
    /// ```
    /// use devolve_ui::core::renderer::renderer::Renderer;
    /// use devolve_ui::rsx;
    /// use std::time::Duration;
    /// use std::thread;
    /// use std::sync::Weak;
    ///
    /// let mut escape: Weak<FlagForOtherThread> = Weak::new();
    /// thread::spawn(move || {
    ///     let renderer = Renderer::new(TODOEngine);
    ///     renderer.root(rsx! { ... });
    ///     renderer.interval_between_frames(Duration::from_millis(25)); // optional
    ///     renderer.show();
    ///     renderer.resume_blocking_with_escape(|e| escape = e);
    /// });
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
            running: RefCell::new(None),
            cached_renders: RefCell::new(HashMap::new()),
            needs_rerender: Arc::new(FlagForOtherThreads::new()),
            root_component: RefCell::new(None)
        });
        let needs_rerender_async = Arc::downgrade(renderer.needs_rerender());
        renderer.engine.borrow_mut().on_resize(Box::new(move || {
            if let Some(needs_rerender) = needs_rerender_async.upgrade() {
                needs_rerender.set();
            }
        }));
        renderer
    }

    #[allow(clippy::needless_lifetimes)] // Not needless
    pub fn needs_rerender<'a>(self: &'a Rc<Self>) -> &'a Arc<FlagForOtherThreads> {
        &self.needs_rerender
    }

    pub fn root(self: &Rc<Self>, construct: impl Fn(&mut Box<VComponent<Engine::ViewData>>) -> VNode<Engine::ViewData> + 'static) {
        self._root(|parent| VComponent::new(parent, &"root".into(), (), move |c, ()| VComponentBody::new(construct(c))))
    }

    fn _root(self: &Rc<Self>, construct: impl FnOnce(VParent<'_, Engine::ViewData>) -> Box<VComponent<Engine::ViewData>>) {
        let self_upcast = self.clone().upcast();
        let root_component = construct(VParent(_VParent::Root(&self_upcast)));
        self.set_root_component(Some(root_component));

        if self.is_visible.get() {
            self.rerender();
        }
    }


    fn set_root_component(self: &Rc<Self>, root_component: Option<Box<VComponent<Engine::ViewData>>>) {
        let mut self_root_component = self.root_component.borrow_mut();
        *self_root_component = root_component;
        if let Some(self_root_component) = self_root_component.as_mut() {
            self_root_component.update(Cow::Borrowed("init:"));
        }
    }

    #[cfg(feature = "time")]
    #[must_use]
    pub fn resume(self: &Rc<Self>) -> RcRunning<Engine> {
        assert!(!self.is_running(), "already running");
        assert!(self.is_visible.get(), "can't resume render while invisible");

        let running = RcRunning::new(self);
        *self.running.borrow_mut() = Some(running.clone());
        running
    }

    #[cfg(feature = "time-blocking")]
    pub fn resume_blocking_with_escape(self: &Rc<Self>, set_escape: impl FnOnce(WeakArc<FlagForOtherThreads>)) {
        let async_runtime = runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        let running = self.resume();
        set_escape(Arc::downgrade(running.is_done()));
        async_runtime.block_on(running);
    }

    #[cfg(feature = "time-blocking")]
    pub fn resume_blocking(self: &Rc<Self>) {
        self.resume_blocking_with_escape(|_| ());
    }

    #[cfg(feature = "time")]
    pub fn pause(self: &Rc<Self>) {
        let running = self.running.take().expect("not running");
        running.is_done.set();
    }

    /// Will be empty if not running
    #[cfg(feature = "time")]
    #[must_use]
    pub fn running(self: &Rc<Self>) -> Weak<Running<Engine>> {
        match self.running.borrow().as_ref() {
            None => Weak::new(),
            Some(running) => Rc::downgrade(&running.0),
        }
    }

    #[cfg(feature = "time")]
    pub fn is_running(self: &Rc<Self>) -> bool {
        self.running.borrow().is_some()
    }

    #[cfg(feature = "time")]
    pub fn set_interval(self: &Rc<Self>, interval_between_frames: Duration) {
        // Need to pause and resume if running to change the interval
        self.interval_between_frames.set(interval_between_frames);
        if let Some(running) = self.running.borrow().as_ref() {
            running.sync_interval();
        }
    }

    /// Make the renderer visible and render once
    pub fn show(self: &Rc<Self>) {
        self.is_visible.set(true);
        self.render(true);
    }

    /// Clear render and make the renderer invisible
    pub fn hide(self: &Rc<Self>) {
        self.clear(true);
        self.is_visible.set(false);
    }

    /// Clears and rerenders immediately
    pub fn rerender(self: &Rc<Self>) {
        assert!(self.is_visible.get(), "can't rerender while invisible");
        assert!(self.root_component.borrow().is_some(), "can't rerender without root component");

        self.clear(false);
        self.render(false);
    }

    fn render(self: &Rc<Self>, is_first: bool) {
        assert!(self.is_visible.get(), "can't render while invisible");
        let borrowed_root_component = self.root_component.borrow();
        let root_component = borrowed_root_component.as_ref().expect("can't render without root component");

        self.needs_rerender.clear();

        let mut engine = self.engine.borrow_mut();
        if is_first {
            engine.start_rendering();
        }

        let root_dimensions = engine.get_root_dimensions();
        let mut render_borrows = RenderBorrows {
            cached_renders: self.cached_renders.borrow_mut(),
            engine
        };

        let final_render = self.render_view(
            root_component.view(),
            &root_dimensions,
            None,
            0,
            0,
            &mut render_borrows
        );
        let mut engine = render_borrows.engine;
        engine.write_render(final_render);
    }

    fn clear(self: &Rc<Self>, is_last: bool) {
        // These assertions can't actually happen
        assert!(self.is_visible.get(), "can't clear render while invisible");
        assert!(self.root_component.borrow().is_some(), "can't clear render without root component");

        let mut engine = self.engine.borrow_mut();
        engine.clear();
        if is_last {
            engine.stop_rendering();
        }
    }

    fn render_view(
        self: &Rc<Self>,
        view: &Box<VView<Engine::ViewData>>,
        parent_bounds: &ParentBounds,
        prev_sibling: Option<&Rectangle>,
        parent_depth: usize,
        sibling_index: usize,
        r: &mut RenderBorrows<'_, Engine>
    ) -> VRender<Engine::RenderLayer> {
        // Try cached
        if let Some(cached_render) = r.cached_renders.get(&view.id()) {
            if &cached_render.parent_bounds == parent_bounds && cached_render.prev_sibling.as_ref() == prev_sibling {
                return cached_render.render.clone();
            } else {
                r.cached_renders.remove(&view.id()).unwrap();
            }
        }

        // Do render
        // Get bounds
        let bounds_result = view.bounds.resolve(parent_bounds, prev_sibling, parent_depth, sibling_index);
        if let Err(error) = bounds_result {
            eprintln!("Error resolving bounds for view {}: {}", view.id(), error);
            return VRender::new();
        }
        let (mut bounding_box, child_store) = bounds_result.unwrap();

        // Render children
        let mut rendered_children: VRender<Engine::RenderLayer> = VRender::new();
        if let Some((children, sub_layout)) = view.d.children() {
            let parent_bounds = ParentBounds {
                bounding_box,
                sub_layout,
                column_size: parent_bounds.column_size.clone(),
                store: child_store
            };
            let mut prev_sibling = None;
            for (sibling_index, child) in children.enumerate() {
                let child_render = self.render_view(
                    child.view(),
                    &parent_bounds,
                    prev_sibling.as_ref(),
                    parent_depth + 1,
                    sibling_index,
                    r
                );
                prev_sibling = child_render.rect().cloned();
                rendered_children.merge(child_render);
            }
            // Move back so borrow checker is happy
            bounding_box = parent_bounds.bounding_box;
        }

        // Resolve size
        /* let inferred_size = bounding_box.with_default_size(&Size {
            width: rendered_children.width(),
            height: rendered_children.height()
        }); */
        if bounding_box.width.is_some_and(|width| width <= 0f32) || bounding_box.height.is_some_and(|height| height <= 0f32) {
            eprintln!("Warning: view has zero or negative dimensions: {} has width={}, height={}", view.id(), bounding_box.width.unwrap_or(f32::NAN), bounding_box.height.unwrap_or(f32::NAN));
        }


        // Render this view
        let render_result = r.engine.make_render(&bounding_box, &parent_bounds.column_size, view, rendered_children);
        render_result.unwrap_or_else(|error| {
            eprintln!("Error rendering view {}: {}", view.id(), error);
            VRender::new()
        })
    }

    fn upcast(self: Rc<Self>) -> Rc<dyn VComponentRoot<ViewData = Engine::ViewData>> {
        self
    }
}

impl <Engine: RenderEngine> VComponentRoot for Renderer<Engine> {
    type ViewData = Engine::ViewData;

    fn invalidate(self: Rc<Self>, view: &Box<VView<Engine::ViewData>>) {
        // Removes this view and all parents from cached_renders
        let mut cached_renders = self.cached_renders.borrow_mut();
        let mut next_view_id = view.id();
        while next_view_id != VNode::<Engine::ViewData>::NULL_ID {
            // This code 1) removes next_view_id from cached_renders,
            // 2) sets next_view_id to the parent (from cached_renders[next_view_id]),
            // and 3) if next_view_id wasn't actually in cached_renders, sets next_view_id to NULL_ID
            // so that the loop breaks.
            next_view_id = cached_renders.remove(&next_view_id)
                .map(|cached_render| cached_render.parent)
                .unwrap_or(VNode::<Engine::ViewData>::NULL_ID);
        }

        self.needs_rerender.set();
    }
}

impl <Engine: RenderEngine> Drop for Renderer<Engine> {
    fn drop(&mut self) {
        if self.is_visible.get() {
            self.engine.borrow_mut().stop_rendering();
        }
    }
}