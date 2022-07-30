use std::rc::Rc;
use crate::component::component::VComponentHead;
use crate::component::path::VComponentPath;
use crate::component::root::VComponentRoot;
use crate::view::view::VViewData;

pub(crate) enum VParent<'a, ViewData: VViewData + ?Sized> {
    Root(&'a Rc<dyn VComponentRoot<ViewData = ViewData>>),
    Component(&'a mut VComponentHead<ViewData>)
}

impl <'a, ViewData: VViewData + ?Sized> VParent<'a, ViewData> {
    pub(crate) fn path(&self) -> &VComponentPath {
        match self {
            VParent::Root(_root) => VComponentPath::ROOT_REF,
            VParent::Component(component) => component.path()
        }
    }
}