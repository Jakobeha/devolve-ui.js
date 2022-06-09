use std::rc::Rc;
use crate::core::component::component::VComponent;
use crate::core::component::path::VNodePath;
use crate::core::component::root::VComponentRoot;
use crate::core::view::view::VViewData;

pub struct VParent<'a, ViewData: VViewData>(pub(in crate::core) _VParent<'a, ViewData>);

pub(in crate::core) enum _VParent<'a, ViewData: VViewData> {
    Root(&'a Rc<dyn VComponentRoot<ViewData = ViewData>>),
    Component(&'a mut Box<VComponent<ViewData>>)
}

impl <'a, ViewData: VViewData> From<&'a mut Box<VComponent<ViewData>>> for VParent<'a, ViewData> {
    fn from(component: &'a mut Box<VComponent<ViewData>>) -> Self {
        VParent(_VParent::Component(component))
    }
}

impl <'a, ViewData: VViewData> VParent<'a, ViewData> {
    pub(in crate::core) fn path(&self) -> VNodePath {
        match &self.0 {
            _VParent::Root(_root) => VNodePath::new(),
            _VParent::Component(component) => component.path()
        }
    }
}