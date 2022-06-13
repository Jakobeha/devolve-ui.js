use std::rc::Rc;
use crate::core::component::component::VComponent;
use crate::core::component::path::VComponentPath;
use crate::core::component::root::VComponentRoot;
use crate::core::view::view::VViewData;

pub(in crate::core) enum VParent<'a, ViewData: VViewData> {
    Root(&'a Rc<dyn VComponentRoot<ViewData = ViewData>>),
    Component(&'a mut Box<VComponent<ViewData>>)
}

impl <'a, ViewData: VViewData> From<&'a mut Box<VComponent<ViewData>>> for VParent<'a, ViewData> {
    fn from(component: &'a mut Box<VComponent<ViewData>>) -> Self {
        VParent::Component(component)
    }
}

impl <'a, ViewData: VViewData> VParent<'a, ViewData> {
    pub(in crate::core) fn path(&self) -> VComponentPath {
        match &self.0 {
            VParent::Root(_root) => VComponentPath::new(),
            VParent::Component(component) => component.path()
        }
    }
}