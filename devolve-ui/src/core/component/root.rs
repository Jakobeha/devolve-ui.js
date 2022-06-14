//! Component root which manages the components. In practice this is always a `Renderer`.
//! This isn't publicly exposed because it's only used internally.

use std::rc::Rc;
use std::sync::{Weak as WeakArc};
#[cfg(feature = "time")]
use std::time::Duration;
use crate::core::component::component::VComponent;
use crate::core::component::path::VComponentPath;
#[cfg(feature = "input")]
use crate::core::misc::input::{KeyEvent, MouseEvent, ResizeEvent};
use crate::core::misc::notify_flag::NotifyFlag;
use crate::core::renderer::listeners::{RendererListener, RendererListenerId};
use crate::core::renderer::stale_data::NeedsUpdateFlag;
use crate::core::view::view::{VView, VViewData};

pub(in crate::core) trait VComponentRoot {
    type ViewData: VViewData;

    /// Mark the view as stale and the given path needs to be updated
    fn invalidate(self: Rc<Self>, path: VComponentPath, view: &Box<VView<Self::ViewData>>);
    /// A flag for a separate thread or time. When set, this marks that the view is stale and the given path
    /// needs to be updated, like `invalidate`
    fn invalidate_flag_for(self: Rc<Self>, path: VComponentPath, view: &Box<VView<Self::ViewData>>) -> WeakArc<NeedsUpdateFlag>;

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
}

impl <ViewData: VViewData> dyn VComponentRoot<ViewData = ViewData> {
    /// Do something with the component at the given path. It will be called with `None` if there is
    /// no component at the given path.
    pub fn with_component(self: Rc<Self>, path: &VComponentPath, fun: impl FnOnce(Option<&mut Box<VComponent<ViewData>>>)) {
        let component = self._with_component(path);
        fun(component.map(|component| unsafe { component.as_mut().unwrap() }))
    }
}