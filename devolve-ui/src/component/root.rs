//! Component root which manages the components. In practice this is always a `Renderer`.
//! This isn't publicly exposed because it's only used internally.

use std::rc::Rc;
#[cfg(feature = "time")]
use std::time::Duration;
#[cfg(feature = "logging")]
use crate::component::mode::VMode;
use crate::component::node::NodeId;
use crate::component::path::{VComponentPath, VComponentRefResolved, VComponentRefResolvedPtr};
use crate::component::update_details::UpdateDetails;
#[cfg(feature = "logging")]
use crate::logging::update_logger::UpdateLogger;
#[cfg(feature = "input")]
use crate::renderer::input::{KeyEvent, MouseEvent, ResizeEvent};
use crate::renderer::listeners::{RendererListener, RendererListenerId};
use crate::renderer::stale_data::{NeedsUpdateFlag, NeedsUpdateNotifier};
use crate::view::view::VViewData;

pub(crate) trait VComponentRoot {
    type ViewData: VViewData;

    /// Marks the given path needs to be updated
    fn queue_needs_update(self: Rc<Self>, path: &VComponentPath, details: UpdateDetails);
    /// Mark the view as stale and needs to be uncached
    fn invalidate_view(self: Rc<Self>, view_id: NodeId);
    /// A flag for a separate thread or time. When set, this marks that the given path needs to be updated, like `mark_needs_update`
    fn needs_update_flag_for(self: Rc<Self>, path: VComponentPath) -> NeedsUpdateFlag;
    /// A flag for a separate thread or time. When set, this marks that an arbitrary path needs to be updated, like `mark_needs_update`
    fn needs_update_notifier(self: Rc<Self>) -> NeedsUpdateNotifier;

    fn _with_component(self: Rc<Self>, path: &VComponentPath) -> Option<VComponentRefResolvedPtr<Self::ViewData>>;

    /// Add a listener for this type of event; used in hooks
    #[cfg(feature = "time")]
    fn listen_for_time(self: Rc<Self>, listener: RendererListener<Duration>) -> RendererListenerId<Duration>;
    /// Remove a listener for this type of event; used in hooks.
    #[cfg(feature = "time")]
    fn unlisten_for_time(self: Rc<Self>, listener_id: RendererListenerId<Duration>);
    /// Add a listener for this type of event; used in hooks
    #[cfg(feature = "input")]
    fn listen_for_keys(self: Rc<Self>, listener: RendererListener<KeyEvent>) -> RendererListenerId<KeyEvent>;
    /// Remove a listener for this type of event; used in hooks.
    #[cfg(feature = "input")]
    fn unlisten_for_keys(self: Rc<Self>, listener_id: RendererListenerId<KeyEvent>);
    /// Add a listener for this type of event; used in hooks
    #[cfg(feature = "input")]
    fn listen_for_mouse(self: Rc<Self>, listener: RendererListener<MouseEvent>) -> RendererListenerId<MouseEvent>;
    /// Remove a listener for this type of event; used in hooks.
    #[cfg(feature = "input")]
    fn unlisten_for_mouse(self: Rc<Self>, listener_id: RendererListenerId<MouseEvent>);
    /// Add a listener for this type of type of event; used in hooks
    #[cfg(feature = "input")]
    fn listen_for_resize(self: Rc<Self>, listener: RendererListener<ResizeEvent>) -> RendererListenerId<ResizeEvent>;
    /// Remove a listener for this event; used in hooks.
    #[cfg(feature = "input")]
    fn unlisten_for_resize(self: Rc<Self>, listener_id: RendererListenerId<ResizeEvent>);

    #[cfg(feature = "logging")]
    fn _with_update_logger(self: Rc<Self>) -> *mut Option<UpdateLogger<Self::ViewData>>;
}

impl <ViewData: VViewData> dyn VComponentRoot<ViewData = ViewData> {
    /// Do something with the component at the given path. It will be called with `None` if there is
    /// no component at the given path.
    pub fn with_component(self: Rc<Self>, path: &VComponentPath, fun: impl FnOnce(Option<VComponentRefResolved<'_, ViewData>>)) {
        if let Some(component) = self._with_component(path) {
            unsafe { component.with_into_mut(|component| fun(Some(component))) }
        } else {
            fun(None)
        }
    }

    /// Do something with the update logger. Logging must be enabled in order for this to be called,
    /// because otherwise you should fastpath and not access the renderer directly.
    #[cfg(feature = "logging")]
    pub fn with_update_logger(self: Rc<Self>, fun: impl FnOnce(&mut UpdateLogger<ViewData>)) {
        assert!(VMode::is_logging(), "VMode::is_logging() not set: check this first so you don't have to access the renderer");
        let logger = unsafe { &mut *self._with_update_logger() };
        if let Some(logger) = logger.as_mut() {
            fun(logger);
        }
    }
}