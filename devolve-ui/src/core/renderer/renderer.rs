use std::borrow::Cow;
use std::cell::{Cell, RefCell, RefMut};
use std::collections::HashMap;
use std::future::Future;
use std::marker::PhantomData;
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
#[cfg(feature = "input")]
use crate::core::misc::input::{KeyEvent, MouseEvent, ResizeEvent};
use crate::core::misc::notify_bool::FlagForOtherThreads;
use crate::core::view::layout::geom::Rectangle;
use crate::core::view::layout::parent_bounds::ParentBounds;
use crate::core::view::view::{VView, VViewData};
use crate::core::renderer::engine::RenderEngine;
use crate::core::renderer::listeners::{RendererListeners, RendererListenerId, RendererListener};
use crate::core::renderer::render::{VRender, VRenderLayer};
use crate::core::renderer::running::{RcRunning, Running, WeakRunning};

struct CachedRender<Layer> {
    render: VRender<Layer>,
    parent_bounds: ParentBounds,
    prev_sibling: Option<Rectangle>,
    parent: NodeId
}

pub struct Renderer<Engine: RenderEngine + 'static> {
    engine: RefCell<Engine>,

    is_visible: Cell<bool>,

    #[cfg(feature = "time")]
    interval_between_frames: Cell<Duration>,
    #[cfg(feature = "time")]
    running: RefCell<Option<RcRunning<Engine>>>,

    listeners: RefCell<RendererListeners>,

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
    /// renderer.set_interval_between_frames(Duration::from_millis(25)); // optional
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
    /// renderer.set_interval_between_frames(Duration::from_millis(25)); // optional
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
    ///     renderer.set_interval_between_frames(Duration::from_millis(25)); // optional
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
            listeners: RefCell::new(RendererListeners::new()),
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

    pub fn is_visible(self: &Rc<Self>) -> bool {
        self.is_visible.get()
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

// region listener methods - these are almost all boilerplate
#[cfg(feature = "time")]
impl <Engine: RenderEngine> Renderer<Engine> where Engine::RenderLayer: VRenderLayer {
    pub fn listen_for_time(&self: Rc<Self>, listener: RendererListener<Duration>) -> RenderListenerId<Duration> {
        let listeners = &mut self.listeners.borrow_mut().time;
        if listeners.is_empty() {
            self.start_listening_for_time();
        }
        listeners.time.add(listener)
    }

    pub fn unlisten_for_time(&self: Rc<Self>, listener_id: RenderListenerId<Duration>) {
        let listeners = &mut self.listeners.borrow_mut().time;
        listeners.time.remove(listener_id);
        if listeners.is_empty() {
            self.stop_listening_for_time();
        }
    }

    pub fn send_time_event(&self: Rc<Self>, time: &Duration) {
        self.listeners.borrow().time.run(time)
    }

    fn start_listening_for_time(&self: Rc<Self>) {
        // Do nothing as of now
    }

    fn stop_listening_for_time(&self: Rc<Self>) {
        // Do nothing as of now
    }
}

#[cfg(feature = "input")]
impl <Engine: RenderEngine> Renderer<Engine> where Engine::RenderLayer: VRenderLayer {
    pub fn listen_for_keys(&self: Rc<Self>, listener: RendererListener<KeyEvent>) -> RenderListenerId<KeyEvent> {
        let listeners = &mut self.listeners.borrow_mut().keys;
        if listeners.is_empty() {
            self.start_listening_for_keys();
        }
        listeners.keys.add(listener)
    }

    pub fn unlisten_for_keys(&self: Rc<Self>, listener_id: RenderListenerId<KeyEvent>) {
        let listeners = &mut self.listeners.borrow_mut().keys;
        listeners.keys.remove(listener_id);
        if listeners.is_empty() {
            self.stop_listening_for_keys();
        }
    }

    pub fn send_key_event(&self: Rc<Self>, event: &KeyEvent) {
        self.listeners.borrow().keys.run(event)
    }

    fn start_listening_for_keys(&self: Rc<Self>) {
        self.engine.borrow_mut().start_listening_for_keys();
    }

    fn stop_listening_for_keys(&self: Rc<Self>) {
        self.engine.borrow_mut().stop_listening_for_keys();
    }

    pub fn listen_for_mouse(&self: Rc<Self>, listener: RendererListener<MouseEvent>) -> RenderListenerId<MouseEvent> {
        let listeners = &mut self.listeners.borrow_mut().mouse;
        if listeners.is_empty() {
            self.start_listening_for_mouse();
        }
        listeners.mouse.add(listener)
    }

    pub fn unlisten_for_mouse(&self: Rc<Self>, listener_id: RenderListenerId<MouseEvent>) {
        let listeners = &mut self.listeners.borrow_mut().mouse;
        listeners.mouse.remove(listener_id);
        if listeners.is_empty() {
            self.stop_listening_for_mouse();
        }
    }

    pub fn send_mouse_event(&self: Rc<Self>, event: &MouseEvent) {
        self.listeners.borrow().mouse.run(event)
    }

    fn start_listening_for_mouse(&self: Rc<Self>) {
        self.engine.borrow_mut().start_listening_for_mouse();
    }

    fn stop_listening_for_mouse(&self: Rc<Self>) {
        self.engine.borrow_mut().stop_listening_for_mouse();
    }

    pub fn listen_for_resize(&self: Rc<Self>, listener: RendererListener<ResizeEvent>) -> RenderListenerId<ResizeEvent> {
        let listeners = &mut self.listeners.borrow_mut().resize;
        if listeners.is_empty() {
            self.start_listening_for_resize();
        }
        listeners.resize.add(listener)
    }

    pub fn unlisten_for_resize(&self: Rc<Self>, listener_id: RenderListenerId<ResizeEvent>) {
        let listeners = &mut self.listeners.borrow_mut().resize;
        listeners.resize.remove(listener_id);
        if listeners.is_empty() {
            self.stop_listening_for_resize();
        }
    }

    pub fn send_resize_event(&self: Rc<Self>, event: &ResizeEvent) {
        self.listeners.borrow().resize.run(event)
    }

    fn start_listening_for_resize(&self: Rc<Self>) {
        self.engine.borrow_mut().start_listening_for_resize();
    }

    fn stop_listening_for_resize(&self: Rc<Self>) {
        self.engine.borrow_mut().stop_listening_for_resize();
    }
}
// endregion

// region time
#[cfg(feature = "time")]
impl <Engine: RenderEngine> Renderer<Engine> where Engine::RenderLayer: VRenderLayer {
    #[must_use]
    pub fn resume(self: &Rc<Self>) -> RcRunning<Engine> {
        assert!(!self.is_running(), "already running");
        assert!(self.is_visible.get(), "can't resume render while invisible");

        let running = RcRunning::new(self);
        *self.running.borrow_mut() = Some(running.clone());
        running
    }

    pub fn pause(self: &Rc<Self>) {
        let running = self.running.take().expect("not running");
        running.is_done.set();
    }

    /// Will be empty if not running
    #[must_use]
    pub fn running(self: &Rc<Self>) -> Weak<Running<Engine>> {
        match self.running.borrow().as_ref() {
            None => Weak::new(),
            Some(running) => Rc::downgrade(&running.0),
        }
    }

    pub fn is_running(self: &Rc<Self>) -> bool {
        self.running.borrow().is_some()
    }

    pub fn interval_between_frames(self: &Rc<Self>) -> Duration {
        self.interval_between_frames.get()
    }

    pub fn set_interval_between_frames(self: &Rc<Self>, interval_between_frames: Duration) {
        // Need to pause and resume if running to change the interval
        self.interval_between_frames.set(interval_between_frames);
        if let Some(running) = self.running.borrow().as_ref() {
            running.sync_interval();
        }
    }
}

#[cfg(feature = "time-blocking")]
impl <Engine: RenderEngine> Renderer<Engine> where Engine::RenderLayer: VRenderLayer {
    pub fn resume_blocking_with_escape(self: &Rc<Self>, set_escape: impl FnOnce(WeakArc<FlagForOtherThreads>)) {
        let async_runtime = runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        let running = self.resume();
        set_escape(Arc::downgrade(running.is_done()));
        async_runtime.block_on(running);
    }

    pub fn resume_blocking(self: &Rc<Self>) {
        self.resume_blocking_with_escape(|_| ());
    }
}
// endregion

// region VComponentRoot impl
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
// endregion

// region Drop impl
impl <Engine: RenderEngine> Drop for Renderer<Engine> {
    fn drop(&mut self) {
        if self.is_visible.get() {
            self.engine.get_mut().stop_rendering();
        }
    }
}
// endregion