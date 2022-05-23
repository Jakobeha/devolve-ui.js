use std::rc::Rc;
use crate::core::component::component::{VComponent, VComponentRoot};
use crate::core::view::view::VViewData;

pub struct VParent<'a, ViewData: VViewData>(pub(in crate::core) _VParent<'a, ViewData>);

pub(in crate::core) enum _VParent<'a, ViewData: VViewData> {
    Root(&'a Rc<dyn VComponentRoot<ViewData = ViewData>>),
    Component(&'a mut VComponent<ViewData>)
}

impl <'a, ViewData: VViewData> From<&'a mut VComponent<ViewData>> for VParent<'a, ViewData> {
    fn from(component: &'a mut VComponent<ViewData>) -> Self {
        VParent(_VParent::Component(component))
    }
}