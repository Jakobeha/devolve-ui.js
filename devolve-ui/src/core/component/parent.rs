use std::rc::Rc;
use crate::core::component::component::VComponentHead;
use crate::core::component::path::VComponentPath;
use crate::core::component::root::VComponentRoot;
use crate::core::view::view::VViewData;

pub(in crate::core) enum VParent<'a, ViewData: VViewData> {
    Root(&'a Rc<dyn VComponentRoot<ViewData = ViewData>>),
    Component(&'a mut VComponentHead<ViewData>)
}

impl <'a, ViewData: VViewData> VParent<'a, ViewData> {
    pub(in crate::core) fn path(&self) -> &VComponentPath {
        match self {
            VParent::Root(_root) => &VComponentPath::ROOT,
            VParent::Component(component) => component.path()
        }
    }
}