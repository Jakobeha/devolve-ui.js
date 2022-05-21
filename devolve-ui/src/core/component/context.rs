use std::rc::Rc;
use crate::core::component::component::{VComponent, VComponentRoot};
use crate::core::view::view::VViewData;

pub enum VParent<'a, ViewData: VViewData> {
    Root(&'a Rc<dyn VComponentRoot<ViewData = ViewData>>),
    Component(&'a mut VComponent<ViewData>)
}