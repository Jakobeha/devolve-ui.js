//! The `Renderer` manages component data, events / listeners, and rendering.
//! It delegates the rendering to its `RenderEngine` (e.g. `TuiRenderEngine` to render to TUIs).
//!
//! ## Runtime
//! The renderer is designed to re-render when component data is updated.
//! However, Rust does not have a common runtime and updates may come in from other threads while
//! the renderer thread is busy. You can choose to either use the renderer's asynchronous runtime,
//! or `poll` the renderer manually (which causes it to flush updates and rerender).
//!
//! - **Built-in fixed-interval runtime:** This is under the `time` feature. You call `resume` or
//!   one of its variants to make the renderer "tick" at a fixed interval (e.g. 25 times per second).
//!   On each tick, the renderer will a) send a tick event, and b) poll itself, flushing any updates
//!   and redrawing. This runtime is implemented via `async` the Tokio runtime, but you can also start
//!   it synchronously: enable the `time-blocking` feature and call `resume_blocking_with_escape` or
//!   `resume_blocking`.
//! - **Manual runtime:** Call `poll` whenever you want the renderer to re-render. The renderer is
//!   neither `Sync` nor `Send` so you must find a way to call into the main thread in order to do this.
//! - **No runtime:** If you use devolve-ui to just render a single frame, you don't need to worry about
//!   updates or runtime.

use std::borrow::Cow;
use std::cell::{Cell, RefCell, RefMut};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::io;
use std::ops::DerefMut;
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Weak as WeakArc};
#[cfg(feature = "time")]
use std::time::{Duration, Instant};
#[cfg(feature = "logging")]
use serde::Serialize;
#[cfg(feature = "time-blocking")]
use tokio::runtime;
use crate::core::component::component::{VComponent, VComponentContexts};
use crate::core::component::context::VComponentContext2;
use crate::core::component::mode::VMode;
use crate::core::component::node::{NodeId, VComponentAndView, VNode};
use crate::core::component::parent::VParent;
use crate::core::component::path::{VComponentPath, VComponentRefResolvedPtr};
use crate::core::component::root::VComponentRoot;
use crate::core::logging::common::LogStart;
use crate::core::logging::render_logger::{RenderLogger, RenderLoggerImpl};
#[cfg(feature = "logging")]
use crate::core::logging::update_logger::UpdateLogger;
#[cfg(feature = "input")]
use crate::core::renderer::input::{KeyEvent, MouseEvent, ResizeEvent};
use crate::core::misc::notify_flag::NotifyFlag;
use crate::core::misc::option_f32::OptionF32;
use crate::core::view::layout::geom::{Rectangle, Size};
use crate::core::view::layout::parent_bounds::{DimsStore, ParentBounds};
use crate::core::view::view::{VView, VViewData};
use crate::core::renderer::engine::RenderEngine;
#[cfg(feature = "input")]
use crate::core::renderer::engine::InputListeners;
use crate::core::renderer::listeners::{RendererListener, RendererListenerId, RendererListeners};
use crate::core::renderer::render::{VRender, VRenderLayer};
use crate::core::renderer::running::RcRunning;
use crate::core::renderer::stale_data::{NeedsRerenderFlag, NeedsUpdateFlag, LocalStaleData, SharedStaleData, NeedsUpdateNotifier};

#[derive(Debug)]
struct CachedRender<Layer> {
    render: VRender<Layer>,
    parent_bounds: ParentBounds,
    prev_sibling: Option<Rectangle>,
    parent: NodeId
}

/// See module-level documentation
pub struct Renderer<Engine: RenderEngine + 'static> {
    engine: RefCell<Engine>,
    overrides: RendererOverrides,

    is_visible: Cell<bool>,
    root_component: RefCell<Option<Box<VComponent<Engine::ViewData>>>>,

    cached_renders: RefCell<HashMap<NodeId, CachedRender<Engine::RenderLayer>>>,
    local_stale_data: LocalStaleData,
    shared_stale_data: Arc<SharedStaleData>,

    #[cfg(feature = "time")]
    interval_between_frames: Cell<Duration>,
    #[cfg(feature = "time")]
    running: RefCell<Option<RcRunning<Engine>>>,
    #[cfg(feature = "time")]
    is_listening_for_time: Cell<bool>,
    #[cfg(feature = "time")]
    last_frame_time: Cell<Option<Instant>>,

    listeners: RefCell<RendererListeners>,
    #[cfg(feature = "input")]
    input_listeners: Cell<InputListeners>,

    #[cfg(feature = "logging")]
    update_logger: RefCell<Option<UpdateLogger<Engine::ViewData>>>,
    #[cfg(feature = "logging")]
    render_logger: RefCell<Option<Box<dyn RenderLogger<ViewData=Engine::ViewData, RenderLayer=Engine::RenderLayer>>>>
}

#[derive(Debug, Default)]
pub struct RendererOverrides {
    pub override_size: Option<Size>,
    pub override_column_size: Option<Size>,
    pub additional_store: DimsStore,
    pub ignore_events: bool
}

struct RenderBorrows<'a, Engine: RenderEngine> {
    pub cached_renders: RefMut<'a, HashMap<NodeId, CachedRender<Engine::RenderLayer>>>,
    pub engine: RefMut<'a, Engine>
}

// region core impls
impl <Engine: RenderEngine> Renderer<Engine> {
    #[cfg(feature = "time")]
    pub const DEFAULT_INTERVAL_BETWEEN_FRAMES: Duration = Duration::from_millis(25);

    /// Creates a new renderer.
    ///
    /// # Examples
    /// Start a renderer which re-renders automatically.
    /// The renderer will fully block the current thread until your app exits.
    /// ```
    /// use devolve_ui::core::renderer::renderer::Renderer;
    /// use std::time::Duration;
    ///
    /// let renderer = Renderer::new(TODOEngine);
    /// renderer.root(|(c, ())| todo_component!(c, "root", ...));
    /// renderer.set_interval_between_frames(Duration::from_millis(25)); // optional
    /// renderer.show();
    /// renderer.resume_blocking();
    /// ```
    ///
    /// Start a renderer which re-renders automatically in an async block.
    /// The renderer will `await` in the current future but not block other concurrent futures.
    /// ```
    /// use devolve_ui::core::renderer::renderer::Renderer;
    /// use std::time::Duration;
    ///
    /// let renderer = Renderer::new(TODOEngine);
    /// renderer.root(|(c, ())| todo_component!(c, "root", ...));
    /// renderer.set_interval_between_frames(Duration::from_millis(25)); // optional
    /// renderer.show();
    /// renderer.resume().await;
    /// ```
    ///
    /// Start a renderer which re-renders on a background thread.
    /// You can stop the renderer by closing the thread, or calling `escape.upgrade.expect("renderer was already stopped").set()`.
    /// ```
    /// use devolve_ui::core::renderer::renderer::Renderer;
    /// use devolve_ui::core::misc::notify_flag::NotifyFlag;
    /// use std::time::Duration;
    /// use std::thread;
    /// use std::sync::Weak;
    ///
    /// let mut escape: Weak<NotifyFlag> = Weak::new();
    /// thread::spawn(move || {
    ///     let renderer = Renderer::new(TODOEngine);
    ///     renderer.root(|(c, ())| todo_component!(c, "root", ...));
    ///     renderer.set_interval_between_frames(Duration::from_millis(25)); // optional
    ///     renderer.show();
    ///     renderer.resume_blocking_with_escape(|e| escape = e);
    /// });
    /// ```
    ///
    /// Start a renderer which re-renders manually (renders once, then can be re-rendered via `renderer.rerender()`)
    /// ```
    /// use devolve_ui::core::renderer::renderer::Renderer;
    /// use std::time::Duration;
    ///
    /// let renderer = Renderer::new(TODOEngine);
    /// renderer.root(|(c, ())| todo_component!(c, "root", ...));
    /// renderer.show();
    /// ```
    pub fn new(engine: Engine) -> Rc<Self> {
        Self::new_with_overrides(engine, RendererOverrides::default())
    }

    pub fn new_with_overrides(engine: Engine, overrides: RendererOverrides) -> Rc<Self> {
        let renderer = Rc::new(Renderer {
            engine: RefCell::new(engine),
            overrides,
            is_visible: Cell::new(false),
            root_component: RefCell::new(None),
            cached_renders: RefCell::new(HashMap::new()),
            local_stale_data: LocalStaleData::new(),
            shared_stale_data: Arc::new(SharedStaleData::new()),
            #[cfg(feature = "time")]
            interval_between_frames: Cell::new(Self::DEFAULT_INTERVAL_BETWEEN_FRAMES),
            #[cfg(feature = "time")]
            running: RefCell::new(None),
            #[cfg(feature = "time")]
            is_listening_for_time: Cell::new(false),
            #[cfg(feature = "time")]
            last_frame_time: Cell::new(None),
            listeners: RefCell::new(RendererListeners::new()),
            #[cfg(feature = "input")]
            input_listeners: Cell::new(InputListeners::empty()),
            #[cfg(feature = "logging")]
            update_logger: RefCell::new(None),
            #[cfg(feature = "logging")]
            render_logger: RefCell::new(None)
        });
        let needs_rerender_flag = renderer.needs_rerender_flag();
        renderer.engine.borrow_mut().on_resize(Box::new(move || {
            needs_rerender_flag.set()
        }));
        renderer
    }

    /// If the renderer currently rendering. If not visible, then there's never a need to rerender.
    pub fn is_visible(self: &Rc<Self>) -> bool {
        self.is_visible.get()
    }

    /// Lets you say on a separate thread that the renderer just needs to rerender,
    /// without specifying a component to update (use `VComponentHead::invalidate` if you want to do that).
    /// This is useful e.g. if your display output changes.
    pub fn needs_rerender_flag(self: &Rc<Self>) -> NeedsRerenderFlag {
        NeedsRerenderFlag::from(&self.shared_stale_data)
    }

    /// If this needs to rerender any components.
    /// Note that this may need to rerender even if no components need updates,
    /// as is the case when the window is resized.
    /// However, if any component needs updates then `needs_rerender()` is guaranteed to be true,
    /// unless the component isn't visible.
    ///
    /// Note that if the renderer needs to rerender because of other threads but `poll` wasn't called,
    /// this will return false: `poll` is needed to sync data from other threads.
    pub fn needs_rerender(self: &Rc<Self>) -> bool {
        self.is_visible() && self.local_stale_data.needs_rerender().get()
    }

    /// Mark that the component needs to rerender (only applies if it's visible)
    fn set_needs_rerender(self: &Rc<Self>) {
        self.local_stale_data.needs_rerender().set();
    }

    /// Mark that the component no longer needs to rerender.
    /// This is only used in `render`, when the component is actually rendered.
    ///
    /// Note that if the renderer needs to rerender because of other threads, this won't clear that flag.
    /// The flag for other threads is cleared in `poll`.
    fn clear_needs_rerender(self: &Rc<Self>) {
        self.local_stale_data.needs_rerender().clear();
    }

    /// Remove a view and all its parents from the render cache.
    fn uncache_view(self: &Rc<Self>, mut view_id: NodeId) {
        let mut cached_renders = self.cached_renders.borrow_mut();
        while view_id != NodeId::NULL {
            // This code 1) removes view_id from cached_renders,
            // 2) sets view_id to the parent (from cached_renders[view_id]),
            // and 3) if view_id wasn't actually in cached_renders, sets view_id to NULL_ID
            // so that the loop breaks.
            view_id = cached_renders.remove(&view_id)
                .map(|cached_render| cached_render.parent)
                .unwrap_or(NodeId::NULL);
        }
    }

    /// Update `local_stale_data` from `shared_stale_data`, and clear the latter.
    fn pull_shared_stale_data(self: &Rc<Self>) {
        let result = self.local_stale_data.append(&self.shared_stale_data);
        if result.is_err() {
            eprintln!("Error pulling update/rerender data from other threads: {:?}", result.unwrap_err());
        }
    }

    /// Just update components. However, then we are rendering old views.
    /// Idk if this should be public. There may be situations where a component needs to update to run side-effects.
    /// But a) that is bad design unless the side-effects are visual, and b) I don't see why anyone
    /// would need a component to run side-effects but not want to render, especially because of a).
    fn update_components(self: &Rc<Self>) {
        if let Some(root_component) = self.root_component.borrow_mut().as_mut() {
            self.local_stale_data.apply_updates(root_component).unwrap();
        }
    }

    /// Determine the root dimensions. This calls `engine.get_root_dimensions` and then sets overrides.
    fn get_root_dimensions(self: &Rc<Self>, engine: &Engine) -> ParentBounds {
        let mut root_dimensions = engine.get_root_dimensions();
        if let Some(override_size) = self.overrides.override_size {
            root_dimensions.bounding_box.width = OptionF32::from(override_size.width);
            root_dimensions.bounding_box.height = OptionF32::from(override_size.height);
        }
        if let Some(override_column_size) = self.overrides.override_column_size {
            root_dimensions.column_size = Cow::Owned(override_column_size.clone());
        }
        root_dimensions.store.append(&mut self.overrides.additional_store.clone());
        root_dimensions
    }

    /// Return this as a `dyn VComponentRoot`. Due to a weird thing in Rust's type system, we need this.
    fn as_vroot(self: Rc<Self>) -> Rc<dyn VComponentRoot<ViewData = Engine::ViewData>> {
        self
    }


    /// Clear render and make the renderer invisible.
    /// Panics if already invisible.
    pub fn hide(self: &Rc<Self>) {
        assert!(self.is_visible.get(), "already hidden");

        self.clear(true);
        self.is_visible.set(false);
    }

    /// Clear render. `is_last` is necessary if the component is about to be hidden / disappear,
    /// as then we need extra cleanup.
    fn clear(&self, is_last: bool) {
        // These assertions can't actually happen
        assert!(self.is_visible.get(), "can't clear render while invisible");
        assert!(self.root_component.borrow().is_some(), "can't clear render without root component");

        let mut engine = self.engine.borrow_mut();
        engine.clear();
        #[cfg(feature = "logging")]
        if let Some(render_logger) = self.render_logger.borrow_mut().as_mut() {
            render_logger.log_clear();
        }

        if is_last {
            engine.stop_rendering();
            #[cfg(feature = "logging")]
            if let Some(render_logger) = self.render_logger.borrow_mut().as_mut() {
                render_logger.log_stop_rendering();
            }
        }
    }
}

impl <Engine: RenderEngine> Renderer<Engine> where Engine::RenderLayer: VRenderLayer {
    /// Pulls shared staled data, updates components, and rerenders if necessary.
    pub fn poll(self: &Rc<Self>) {
        self.pull_shared_stale_data();

        self.update_components();
        if self.needs_rerender() {
            self.rerender();
        }
    }

    /// Assign a root component to the renderer.
    /// Before a root component is assigned, the renderer is empty and trying to show will panic.
    /// You can assign another root component after and it will be replaced and re-render.
    pub fn root(self: &Rc<Self>, construct: impl Fn(VComponentContext2<'_, '_, (), Engine::ViewData>) -> VNode<Engine::ViewData> + 'static) {
        self._root(|parent| VComponent::new(parent, &mut VComponentContexts::new(), ().into(), (), construct))
    }

    /// Actually root. When the user calls `root`, we actually assign another root component under.
    /// This is because the function to render components takes a `VParent` but this struct is private
    /// from the user: it's private because the case will always be `VParent::Component` except for
    /// the root component, and if we just make the root component here it can always be private.
    ///
    /// It reduces the API footprint but is it good design?
    fn _root(self: &Rc<Self>, construct: impl FnOnce(VParent<'_, Engine::ViewData>) -> Box<VComponent<Engine::ViewData>>) {
        let self_as_vroot = self.clone().as_vroot();
        let root_component = construct(VParent::Root(&self_as_vroot));
        self.set_root_component(Some(root_component));
    }

    /// Ok, *actually* root. The last function takes a `construct` closure which creates the root component.
    /// This one takes the already-created root component. We set the root component in `self`.
    /// Then update it (runs any pending updates), and rerender if visible, as if the component was
    /// "invalid" (because it is) and we called `update_and_rerender` (which we did not, but updating
    /// and rerendering every time we reroot is a sensible decision. Why would you want to reroot
    /// but not update or rerender?)
    fn set_root_component(self: &Rc<Self>, root_component: Option<Box<VComponent<Engine::ViewData>>>) {
        // Set root component
        let mut self_root_component = self.root_component.borrow_mut();
        *self_root_component = root_component;

        // Update
        if let Some(self_root_component) = self_root_component.as_mut() {
            self_root_component.update(&mut VComponentContexts::new())
        }

        // Rerender
        if self.is_visible.get() {
            self.rerender();
        }
    }

    /// Make the renderer visible and render once.
    /// Panics if already visible or not rooted.
    pub fn show(self: &Rc<Self>) {
        assert!(!self.is_visible.get(), "already visible");
        assert!(self.root_component.borrow().is_some(), "can't show without root component");

        self.is_visible.set(true);
        self.render(true);
    }

    /// Clears and rerenders immediately, even if `needs_rerender()` is false.
    /// Panics of not already shown via `show`.
    pub fn rerender(self: &Rc<Self>) {
        assert!(self.is_visible.get(), "can't rerender while invisible");
        // Probably can't happen
        assert!(self.root_component.borrow().is_some(), "can't rerender without root component");

        self.clear(false);
        self.render(false);
    }

    /// Renders. Does not clear, even if `is_first` is false.
    /// However `is_first` is necessary sometimes for some stuff we only have to do once.
    fn render(self: &Rc<Self>, is_first: bool) {
        assert!(self.is_visible.get(), "can't render while invisible");
        let borrowed_root_component = self.root_component.borrow();
        let root_component = &borrowed_root_component.as_ref().expect("can't render without root component").head;

        self.clear_needs_rerender();

        let mut engine = self.engine.borrow_mut();
        if is_first {
            engine.start_rendering();
            #[cfg(feature = "logging")]
            if let Some(render_logger) = self.render_logger.borrow_mut().as_mut() {
                render_logger.log_start_rendering();
            }
        }

        let root_dimensions = self.get_root_dimensions(&engine);
        let mut render_borrows = RenderBorrows {
            cached_renders: self.cached_renders.borrow_mut(),
            engine
        };

        let final_render = self.render_view(
            root_component.component_and_view(),
            NodeId::NULL,
            &root_dimensions,
            None,
            0,
            0,
            &mut render_borrows
        );

        let mut engine = render_borrows.engine;
        engine.write_render(final_render);
        #[cfg(feature = "logging")]
        if let Some(render_logger) = self.render_logger.borrow_mut().as_mut() {
            render_logger.log_write_render();
        }
    }

    /// Add a view to the render cache.
    fn cache_view(
        self: &Rc<Self>,
        view: &Box<VView<Engine::ViewData>>,
        parent_id: NodeId,
        parent_bounds: &ParentBounds,
        prev_sibling: Option<&Rectangle>,
        render: VRender<Engine::RenderLayer>,
        r: &mut RenderBorrows<'_, Engine>
    ) {
        let cached_renders = &mut r.cached_renders;
        let old_cached_render = cached_renders.insert(view.id(), CachedRender {
            parent_bounds: parent_bounds.clone(),
            prev_sibling: prev_sibling.cloned(),
            parent: parent_id,
            render,
        });
        assert!(old_cached_render.is_none(), "sanity check failed: we cached a view when we already had a cache for its id");
    }

    /// Render a view. First we try cached, otherwise we actually render.
    /// As you can see, there is a lot of context needed to render the view:
    /// parent information, sibling information, the view's component,
    /// and some borrowed data so we don't keep calling `RefCell` and panic in recursive renders.
    fn render_view(
        self: &Rc<Self>,
        (c, view): VComponentAndView<'_, Engine::ViewData>,
        parent_id: NodeId,
        parent_bounds: &ParentBounds,
        prev_sibling: Option<&Rectangle>,
        parent_depth: usize,
        sibling_index: usize,
        r: &mut RenderBorrows<'_, Engine>
    ) -> VRender<Engine::RenderLayer> {
        self.cached_render_view_or(
            (c, view),
            parent_id,
            parent_bounds,
            prev_sibling,
            r,
            |(c, view), parent_bounds, prev_sibling, r| {
                self.force_render_view(
                    (c, view),
                    parent_bounds,
                    prev_sibling,
                    parent_depth,
                    sibling_index,
                    r
                )
            }
        )
    }

    /// If the view is cached and the data to render it isn't stale,
    /// then returns the cached render. Otherwise removes the cached render,
    /// calls `do_render` (to actually render), and then updates the cache with that result
    /// before returning it.
    fn cached_render_view_or(
        self: &Rc<Self>,
        (c, view): VComponentAndView<'_, Engine::ViewData>,
        parent_id: NodeId,
        parent_bounds: &ParentBounds,
        prev_sibling: Option<&Rectangle>,
        r: &mut RenderBorrows<'_, Engine>,
        do_render: impl FnOnce(
            VComponentAndView<'_, Engine::ViewData>,
            &ParentBounds,
            Option<&Rectangle>,
            &mut RenderBorrows<'_, Engine>
        ) -> VRender<Engine::RenderLayer>
    ) -> VRender<Engine::RenderLayer> {
        // Try cached
        if let Some(cached_render) = r.cached_renders.get(&view.id()) {
            if &cached_render.parent_bounds == parent_bounds && cached_render.prev_sibling.as_ref() == prev_sibling {
                let render = cached_render.render.clone();

                #[cfg(feature = "logging")]
                if let Some(render_logger) = self.render_logger.borrow_mut().as_mut() {
                    render_logger.log_render_view(
                        (c, view),
                        parent_id,
                        parent_bounds,
                        prev_sibling,
                        &render,
                        true
                    );
                }

                return render;
            } else {
                r.cached_renders.remove(&view.id()).unwrap();
            }
        }

        let render = do_render(
            (c, view),
            parent_bounds,
            prev_sibling,
            r
        );

        self.cache_view(view, parent_id, parent_bounds, prev_sibling, render.clone(), r);

        #[cfg(feature = "logging")]
        if let Some(render_logger) = self.render_logger.borrow_mut().as_mut() {
            render_logger.log_render_view(
                (c, view),
                parent_id,
                parent_bounds,
                prev_sibling,
                &render,
                false
            );
        }

        render
    }

    /// Render the view without trying to cache it or touching the cache.
    /// Except we will try / update cached when rendering children.
    fn force_render_view(
        self: &Rc<Self>,
        (c, view): VComponentAndView<'_, Engine::ViewData>,
        parent_bounds: &ParentBounds,
        prev_sibling: Option<&Rectangle>,
        parent_depth: usize,
        sibling_index: usize,
        r: &mut RenderBorrows<'_, Engine>
    ) -> VRender<Engine::RenderLayer> {
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
            let child_parent_id = view.id();
            let child_parent_bounds = ParentBounds {
                bounding_box,
                sub_layout,
                column_size: parent_bounds.column_size.clone(),
                store: child_store
            };
            let mut child_prev_sibling = None;
            let child_parent_depth = parent_depth + 1;
            for (child_sibling_index, child) in children.enumerate() {
                let child_render = self.render_view(
                    child.component_and_view(c),
                    child_parent_id,
                    &child_parent_bounds,
                    child_prev_sibling.as_ref(),
                    child_parent_depth,
                    child_sibling_index,
                    r
                );
                child_prev_sibling = child_render.rect().cloned();
                rendered_children.merge(child_render);
            }
            // Move back so borrow checker is happy
            bounding_box = child_parent_bounds.bounding_box;
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
}
// endregion

// region listener methods - these are almost all boilerplate
#[cfg(feature = "time")]
impl <Engine: RenderEngine> Renderer<Engine> {
    fn _listen_for_time(self: &Rc<Self>, listener: RendererListener<Duration>) -> RendererListenerId<Duration> {
        let listeners = &mut self.listeners.borrow_mut().time;
        if listeners.is_empty() {
            self.start_listening_for_time();
        }
        listeners.add(listener)
    }

    fn _unlisten_for_time(self: &Rc<Self>, listener_id: RendererListenerId<Duration>) {
        let listeners = &mut self.listeners.borrow_mut().time;
        listeners.remove(listener_id);
        if listeners.is_empty() {
            self.stop_listening_for_time();
        }
    }

    /// Send time event to all listeners.
    /// If running aka `resume`d, this will send time events.
    /// You use this directly e.g. to send fake time events,
    /// or just make your own runtime instead of using ours.
    pub fn send_time_event(self: &Rc<Self>, time: &Duration) {
        self.listeners.borrow().time.run(time)
    }

    fn start_listening_for_time(self: &Rc<Self>) {
        self.last_frame_time.set(Some(Instant::now()));
        self.is_listening_for_time.set(true);
    }

    fn stop_listening_for_time(self: &Rc<Self>) {
        self.last_frame_time.set(None);
        self.is_listening_for_time.set(false);
    }
}

#[cfg(feature = "input")]
impl <Engine: RenderEngine> Renderer<Engine> {
    fn _listen_for_keys(self: &Rc<Self>, listener: RendererListener<KeyEvent>) -> RendererListenerId<KeyEvent> {
        let listeners = &mut self.listeners.borrow_mut().keys;
        if listeners.is_empty() {
            self.start_listening_for_keys();
        }
        listeners.add(listener)
    }

    fn _unlisten_for_keys(self: &Rc<Self>, listener_id: RendererListenerId<KeyEvent>) {
        let listeners = &mut self.listeners.borrow_mut().keys;
        listeners.remove(listener_id);
        if listeners.is_empty() {
            self.stop_listening_for_keys();
        }
    }

    fn _listen_for_mouse(self: &Rc<Self>, listener: RendererListener<MouseEvent>) -> RendererListenerId<MouseEvent> {
        let listeners = &mut self.listeners.borrow_mut().mouse;
        if listeners.is_empty() {
            self.start_listening_for_mouse();
        }
        listeners.add(listener)
    }

    fn _unlisten_for_mouse(self: &Rc<Self>, listener_id: RendererListenerId<MouseEvent>) {
        let listeners = &mut self.listeners.borrow_mut().mouse;
        listeners.remove(listener_id);
        if listeners.is_empty() {
            self.stop_listening_for_mouse();
        }
    }

    fn _listen_for_resize(self: &Rc<Self>, listener: RendererListener<ResizeEvent>) -> RendererListenerId<ResizeEvent> {
        let listeners = &mut self.listeners.borrow_mut().resize;
        if listeners.is_empty() {
            self.start_listening_for_resize();
        }
        listeners.add(listener)
    }

    fn _unlisten_for_resize(self: &Rc<Self>, listener_id: RendererListenerId<ResizeEvent>) {
        let listeners = &mut self.listeners.borrow_mut().resize;
        listeners.remove(listener_id);
        if listeners.is_empty() {
            self.stop_listening_for_resize();
        }
    }

    /// Send key event to all listeners.
    /// The render engine will send events, you use this directly e.g. to send a fake key event in a test.
    pub fn send_key_event(self: &Rc<Self>, event: &KeyEvent) {
        self.listeners.borrow().keys.run(event)
    }

    fn start_listening_for_keys(self: &Rc<Self>) {
        self.input_listeners.set(self.input_listeners.get() | InputListeners::KEYS);
        self.update_input_listeners();
    }

    fn stop_listening_for_keys(self: &Rc<Self>) {
        self.input_listeners.set(self.input_listeners.get() | InputListeners::KEYS);
        self.update_input_listeners();
    }

    /// Send mouse event to all listeners.
    pub fn send_mouse_event(self: &Rc<Self>, event: &MouseEvent) {
        self.listeners.borrow().mouse.run(event)
    }

    fn start_listening_for_mouse(self: &Rc<Self>) {
        self.input_listeners.set(self.input_listeners.get() | InputListeners::MOUSE);
        self.update_input_listeners();
    }

    fn stop_listening_for_mouse(self: &Rc<Self>) {
        self.input_listeners.set(self.input_listeners.get() - InputListeners::MOUSE);
        self.update_input_listeners();
    }

    /// Send resize event to all listeners.
    /// If running, this *will* trigger a rerender. None of the others do that.
    pub fn send_resize_event(self: &Rc<Self>, event: &ResizeEvent) {
        self.set_needs_rerender();
        self.listeners.borrow().resize.run(event)
    }

    fn start_listening_for_resize(self: &Rc<Self>) {
        self.input_listeners.set(self.input_listeners.get() | InputListeners::RESIZE);
        self.update_input_listeners();
    }

    fn stop_listening_for_resize(self: &Rc<Self>) {
        self.input_listeners.set(self.input_listeners.get() - InputListeners::RESIZE);
        self.update_input_listeners();
    }

    fn update_input_listeners(self: &Rc<Self>) {
        if !self.overrides.ignore_events {
            self.engine.borrow_mut().update_input_listeners(self.input_listeners.get());
        }
    }
}
// endregion

// region time
#[cfg(feature = "time")]
impl <Engine: RenderEngine> Renderer<Engine> {
    /// If running, the renderer will rerender on its own at a constant rate some FPS.
    pub fn is_running(self: &Rc<Self>) -> bool {
        self.running.borrow().is_some()
    }

    /// The interval the renderer rerenders, if it's running.
    pub fn interval_between_frames(self: &Rc<Self>) -> Duration {
        self.interval_between_frames.get()
    }

    /// Change the interval the renderer renders when running
    /// (can call this when not running or when running).
    pub fn set_interval_between_frames(self: &Rc<Self>, interval_between_frames: Duration) {
        // Need to pause and resume if running to change the interval
        self.interval_between_frames.set(interval_between_frames);
        if let Some(running) = self.running.borrow().as_ref() {
            running.sync_interval();
        }
    }

    /// Send tick events to the engine and listeners.
    pub(super) fn tick(self: &Rc<Self>) {
        self.engine.borrow_mut().tick(RendererViewForEngineInTick(self));

        if self.is_listening_for_time.get() {
            let delta_time = self.last_frame_time
                .get()
                .expect("invalid state: renderer is_listening_for_time but no last_frame_time")
                .elapsed();
            self.last_frame_time.set(Some(Instant::now()));
            self.listeners.borrow().time.run(&delta_time);
        }
    }
}

#[cfg(feature = "time")]
impl <Engine: RenderEngine> Renderer<Engine> where Engine::RenderLayer: VRenderLayer {
    /// Starts running the renderer: that is, the renderer will start rerendering at a constant interval.
    /// This is for async contexts and returns a `Future` which will resolve when the renderer stops running.
    /// Call `resume_blocking_...` if you aren't doing this in an async context.
    #[must_use]
    pub fn resume(self: &Rc<Self>) -> RcRunning<Engine> {
        assert!(!self.is_running(), "already running");
        assert!(self.is_visible.get(), "can't resume render while invisible");

        let running = RcRunning::new(self);
        *self.running.borrow_mut() = Some(running.clone());
        running
    }

    /// Stop running the renderer. It doesn't clear the current render. It will cause the call from
    /// `resume` to end.
    pub fn pause(self: &Rc<Self>) {
        let running = self.running.take().expect("not running");
        running.is_done.set();
    }
}

#[cfg(feature = "time-blocking")]
impl <Engine: RenderEngine> Renderer<Engine> where Engine::RenderLayer: VRenderLayer {
    /// Starts running the renderer on the current thread, blocking it until the renderer is paused.
    /// However, `set_escape` is called first and you can use it to set a `WeakArc` for another thread,
    /// and then upgrade that value and call `set` to pause the renderer from the other thread.
    ///
    /// ## Examples
    /// Run the renderer on this thread and then stop after 1 second.
    /// ```
    /// use devolve_ui::core::renderer::renderer::Renderer;
    /// use devolve_ui::core::misc::notify_flag::NotifyFlag;
    /// use std::time::Duration;
    /// use std::thread;
    /// use std::sync::Weak;
    ///
    /// let mut escape: Weak<NotifyFlag> = Weak::new();
    /// thread::spawn(move || {
    ///     thread::sleep(Duration::from_secs(1));
    ///
    ///     // Our scheduler is terrible and this should never happen after 1 second
    ///     // but it's usually a good idea to not have data races
    ///     while escape.upgrade().is_none() {
    ///       thread::sleep(Duration::from_millis(1));
    ///     }
    ///
    ///     escape.upgrade().unwrap().set();
    /// });
    ///
    /// let renderer = Renderer::new(TODOEngine);
    /// renderer.root(|(c, ())| todo_component!(c, "root", ...));
    /// renderer.show();
    /// renderer.resume_blocking_with_escape(|e| escape = e);
    ///
    /// ```
    ///
    /// Run the renderer on another thread and then stop after 1 second
    /// ```
    /// use devolve_ui::core::renderer::renderer::Renderer;
    /// use devolve_ui::core::misc::notify_flag::NotifyFlag;
    /// use std::time::Duration;
    /// use std::thread;
    /// use std::sync::Weak;
    ///
    /// let mut escape: Weak<NotifyFlag> = Weak::new();
    /// thread::spawn(move || {
    ///     let renderer = Renderer::new(TODOEngine);
    ///     renderer.root(|(c, ())| todo_component!(c, "root", ...));
    ///     renderer.show();
    ///     renderer.resume_blocking_with_escape(|e| escape = e);
    /// });
    ///
    /// thread::sleep(Duration::from_secs(1));
    /// escape.upgrade().unwrap().set()
    /// ```
    pub fn resume_blocking_with_escape(self: &Rc<Self>, set_escape: impl FnOnce(WeakArc<NotifyFlag>)) {
        let async_runtime = runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        let running = self.resume();
        set_escape(Arc::downgrade(running.is_done()));
        async_runtime.block_on(running);
    }

    /// Starts running the renderer on the current thread, blocking permanently.
    /// (Well technically not permanently if the renderer gets paused.
    /// However, there is no way to pause the renderer from within `VComponents` publicly
    /// so at least until a later API version it's permanent).
    pub fn resume_blocking(self: &Rc<Self>) {
        self.resume_blocking_with_escape(|_| ());
    }
}

/// We cannot provide all of the renderer's methods to the engine because there are situations
/// the engine could create a `RefCell` runtime error since it's being borrowed.
/// The functions visible in this struct should never cause an error.
///
/// Additionally, we use this to provide special private functionality to `RenderEngine` during tick,
/// and restrict functionality to only that which the engine should actually need.
#[cfg(feature = "time")]
pub struct RendererViewForEngineInTick<'a, Engine: RenderEngine + 'static>(&'a Rc<Renderer<Engine>>);

#[cfg(feature = "time")]
impl <'a, Engine: RenderEngine + 'static> RendererViewForEngineInTick<'a, Engine> {
    /// Send a key event to listeners.
    pub fn send_key_event(&self, event: &KeyEvent) {
        self.0.send_key_event(event)
    }

    /// Send a mouse event to listeners.
    pub fn send_mouse_event(&self, event: &MouseEvent) {
        self.0.send_mouse_event(event)
    }

    /// Send a resize event to listeners.
    /// If running, this *will* trigger a rerender.
    pub fn send_resize_event(&self, event: &ResizeEvent) {
        self.0.send_resize_event(event)
    }

    /// Notify that we need to rerender but no specific components need updates.
    pub fn set_needs_rerender(&self) {
        self.0.set_needs_rerender()
    }
}
// endregion

// region logging
#[cfg(feature = "logging")]
impl <Engine: RenderEngine> Renderer<Engine> where Engine::ViewData: Serialize + Debug + Clone, Engine::RenderLayer: Serialize + Debug {
    fn set_update_logger(self: &Rc<Self>, logger: Option<UpdateLogger<Engine::ViewData>>) {
        *self.update_logger.borrow_mut() = logger;
    }

    fn set_render_logger(self: &Rc<Self>, logger: Option<Box<dyn RenderLogger<ViewData=Engine::ViewData, RenderLayer=Engine::RenderLayer>>>) {
        *self.render_logger.borrow_mut() = logger;
    }

    pub fn enable_logging(self: &Rc<Self>, dir: &Path) -> io::Result<()> {
        assert!(self.update_logger.borrow().is_none(), "already logging");
        VMode::set_is_logging(true);
        let log_start = LogStart::try_new(dir)?;
        let update_logger = UpdateLogger::try_new(&log_start)?;
        let render_logger = RenderLoggerImpl::try_new(&log_start)?;
        self.set_update_logger(Some(update_logger));
        self.set_render_logger(Some(Box::new(render_logger)));
        Ok(())
    }

    pub fn disable_logging(self: &Rc<Self>) {
        assert!(self.update_logger.borrow().is_some(), "not logging");
        self.set_update_logger(None);
        self.set_render_logger(None);
    }
}
// endregion

// region VComponentRoot impl
impl <Engine: RenderEngine> VComponentRoot for Renderer<Engine> {
    type ViewData = Engine::ViewData;

    fn queue_needs_update(self: Rc<Self>, path: &VComponentPath) {
        self.local_stale_data.queue_path_for_update_no_details(path).unwrap();
        self.set_needs_rerender();
    }

    fn invalidate_view(self: Rc<Self>, view: &Box<VView<Engine::ViewData>>) {
        self.uncache_view(view.id());
    }

    fn needs_update_flag_for(self: Rc<Self>, path: VComponentPath) -> NeedsUpdateFlag {
        NeedsUpdateFlag::from(&self.shared_stale_data, path)
    }

    fn needs_update_notifier(self: Rc<Self>) -> NeedsUpdateNotifier {
        NeedsUpdateNotifier::from(&self.shared_stale_data)
    }

    fn _with_component(self: Rc<Self>, path: &VComponentPath) -> Option<VComponentRefResolvedPtr<Self::ViewData>> {
        self.root_component.borrow_mut().as_mut()
            .and_then(|root| root.down_path_mut(path, Vec::new()))
            .map(|component| component.into_ptr())
    }

    #[cfg(feature = "time")]
    fn listen_for_time(self: Rc<Self>, listener: RendererListener<Duration>) -> RendererListenerId<Duration> {
        self._listen_for_time(listener)
    }

    #[cfg(feature = "time")]
    fn unlisten_for_time(self: Rc<Self>, listener_id: RendererListenerId<Duration>) {
        self._unlisten_for_time(listener_id)
    }

    #[cfg(feature = "input")]
    fn listen_for_keys(self: Rc<Self>, listener: RendererListener<KeyEvent>) -> RendererListenerId<KeyEvent> {
        self._listen_for_keys(listener)
    }

    #[cfg(feature = "input")]
    fn unlisten_for_keys(self: Rc<Self>, listener_id: RendererListenerId<KeyEvent>) {
        self._unlisten_for_keys(listener_id)
    }

    #[cfg(feature = "input")]
    fn listen_for_mouse(self: Rc<Self>, listener: RendererListener<MouseEvent>) -> RendererListenerId<MouseEvent> {
        self._listen_for_mouse(listener)
    }

    #[cfg(feature = "input")]
    fn unlisten_for_mouse(self: Rc<Self>, listener_id: RendererListenerId<MouseEvent>) {
        self._unlisten_for_mouse(listener_id)
    }

    #[cfg(feature = "input")]
    fn listen_for_resize(self: Rc<Self>, listener: RendererListener<ResizeEvent>) -> RendererListenerId<ResizeEvent> {
        self._listen_for_resize(listener)
    }

    #[cfg(feature = "input")]
    fn unlisten_for_resize(self: Rc<Self>, listener_id: RendererListenerId<ResizeEvent>) {
        self._unlisten_for_resize(listener_id)
    }

    #[cfg(feature = "logging")]
    fn _with_update_logger(self: Rc<Self>) -> *mut Option<UpdateLogger<Self::ViewData>> {
        self.update_logger.borrow_mut().deref_mut() as *mut _
    }
}
// endregion

// region Drop impl
impl <Engine: RenderEngine> Drop for Renderer<Engine> {
    fn drop(&mut self) {
        if self.is_visible.get() {
            self.clear(true);
        }
    }
}
// endregion

// region Debug impl
impl <Engine: RenderEngine + Debug> Debug for Renderer<Engine> where Engine::ViewData: Debug, Engine::RenderLayer: Debug {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Renderer")
            .field("root_component", &self.root_component.borrow())
            .field("cached_renders", &self.cached_renders.borrow())
            .field("is_visible", &self.is_visible.get())
            .field("local_stale_data", &self.local_stale_data)
            .field("shared_stale_data", &self.shared_stale_data)
            .field("engine", &self.engine.borrow())
            .finish_non_exhaustive()
    }
}
// endregion