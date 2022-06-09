use std::rc::Rc;
#[cfg(feature = "time")]
use std::time::Duration;
use crate::core::component::component::VComponent;
use crate::core::component::path::VNodePath;
#[cfg(feature = "input")]
use crate::core::misc::input::{KeyEvent, MouseEvent, ResizeEvent};
use crate::core::renderer::listeners::{RendererListener, RendererListenerId};
use crate::core::view::view::{VView, VViewData};

pub(in crate::core) trait VComponentRoot {
    type ViewData: VViewData;

    fn invalidate(self: Rc<Self>, view: &Box<VView<Self::ViewData>>);

    fn with_component<'a>(self: Rc<Self>, path: &VNodePath, fun: Box<dyn FnOnce(Option<&mut Box<VComponent<Self::ViewData>>>) + 'a>);

    #[cfg(feature = "time")]
    fn listen_for_time(self: Rc<Self>, listener: RendererListener<Duration>) -> RendererListenerId<Duration>;
    #[cfg(feature = "time")]
    fn unlisten_for_time(self: Rc<Self>, listener_id: RendererListenerId<Duration>);
    #[cfg(feature = "input")]
    fn listen_for_keys(self: Rc<Self>, listener: RendererListener<KeyEvent>) -> RendererListenerId<KeyEvent>;
    #[cfg(feature = "input")]
    fn unlisten_for_keys(self: Rc<Self>, listener_id: RendererListenerId<KeyEvent>);
    #[cfg(feature = "input")]
    fn listen_for_mouse(self: Rc<Self>, listener: RendererListener<MouseEvent>) -> RendererListenerId<MouseEvent>;
    #[cfg(feature = "input")]
    fn unlisten_for_mouse(self: Rc<Self>, listener_id: RendererListenerId<MouseEvent>);
    #[cfg(feature = "input")]
    fn listen_for_resize(self: Rc<Self>, listener: RendererListener<ResizeEvent>) -> RendererListenerId<ResizeEvent>;
    #[cfg(feature = "input")]
    fn unlisten_for_resize(self: Rc<Self>, listener_id: RendererListenerId<ResizeEvent>);
}
