//! Component root which manages the components. In practice this is always a `Renderer`.
//! This isn't publicly exposed because it's only used internally.

#[cfg(feature = "logging")]
use std::cell::RefMut;
use std::rc::Rc;
#[cfg(feature = "time")]
use std::time::Duration;
use crate::core::component::component::VComponent;
#[cfg(feature = "logging")]
use crate::core::component::mode::VMode;
use crate::core::component::path::VComponentPath;
#[cfg(feature = "logging")]
use crate::core::logging::update_logger::UpdateLogger;
#[cfg(feature = "input")]
use crate::core::renderer::input::{KeyEvent, MouseEvent, ResizeEvent};
use crate::core::renderer::listeners::{RendererListener, RendererListenerId};
use crate::core::renderer::stale_data::NeedsUpdateFlag;
use crate::core::view::view::{VView, VViewData};

pub(in crate::core) trait VComponentRoot {
    type ViewData: VViewData;

    /// Mark the view as stale and the given path needs to be updated
    fn invalidate(self: Rc<Self>, path: VComponentPath, view: &Box<VView<Self::ViewData>>);
    /// A flag for a separate thread or time. When set, this marks that the view is stale and the given path
    /// needs to be updated, like `invalidate`
    fn invalidate_flag_for(self: Rc<Self>, path: VComponentPath, view: &Box<VView<Self::ViewData>>) -> NeedsUpdateFlag;

    fn _with_component(self: Rc<Self>, path: &VComponentPath) -> Option<*mut Box<VComponent<Self::ViewData>>>;

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
    fn _with_update_logger(self: Rc<Self>) -> RefMut<'_, Option<UpdateLogger<Self::ViewData>>>;
}

impl <ViewData: VViewData> dyn VComponentRoot<ViewData = ViewData> {
    /// Do something with the component at the given path. It will be called with `None` if there is
    /// no component at the given path.
    pub fn with_component(self: Rc<Self>, path: &VComponentPath, fun: impl FnOnce(Option<&mut Box<VComponent<ViewData>>>)) {
        if let Some(component) = self._with_component(path) {
            fun(Some(unsafe { component.as_mut().unwrap() }))
        } else {
            fun(None)
        }
    }

    /// Do something with the update logger. Logging must be enabled in order for this to be called,
    /// because otherwise you should fastpath and not access the renderer directly.
    #[cfg(feature = "logging")]
    pub fn with_update_logger(self: Rc<Self>, fun: impl FnOnce(&mut UpdateLogger<ViewData>)) {
        assert!(VMode::is_logging(), "VMode::is_logging() not set: check this first so you don't have to access the renderer");
        let mut logger = self._with_update_logger();
        if let Some(logger) = logger.as_mut() {
            fun(logger);
        }
    }
}