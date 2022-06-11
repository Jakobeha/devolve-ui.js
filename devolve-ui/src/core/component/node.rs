//! A `VNode` is either a component of a view. Either way, the node contains content which is rendered,
//! and may contain child nodes.

use std::borrow::Cow;
use std::fmt;
use std::fmt::{Display, Formatter};
use crate::core::component::component::VComponent;
use crate::core::component::path::VComponentKey;
use crate::core::view::view::{VView, VViewData};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct NodeId(usize);

impl Display for NodeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug)]
pub enum VNode<ViewData: VViewData> {
    Component { id: NodeId, key: VComponentKey },
    View(Box<VView<ViewData>>)
}

#[derive(Debug)]
pub enum VNodeResolved<'c, 'v, ViewData: VViewData> {
    Component(&'c Box<VComponent<ViewData>>),
    View(&'v Box<VView<ViewData>>)
}

#[derive(Debug)]
pub enum VNodeResolvedMut<'c, 'v, ViewData: VViewData> {
    Component(&'c mut Box<VComponent<ViewData>>),
    View(&'v mut Box<VView<ViewData>>)
}

pub type VComponentAndView<'a, ViewData> = (&'a Box<VComponent<ViewData>>, &'a Box<VView<ViewData>>);

static mut NEXT_ID: usize = 0;

impl <ViewData: VViewData> VNode<ViewData> {
    pub const NULL_ID: NodeId = NodeId(0);

    pub fn next_id() -> NodeId {
        // TODO: Make thread safe?
        unsafe {
            NEXT_ID += 1;
            NodeId(NEXT_ID)
        }
    }

    pub fn id(&self) -> NodeId {
        match self {
            VNode::Component { id, key: _key} => *id,
            VNode::View(view) => view.id()
        }
    }

    pub fn resolve<'c, 'v>(&'v self, parent: &'c Box<VComponent<ViewData>>) -> VNodeResolved<'c, 'v, ViewData> {
        match self {
            VNode::Component { id: _id, key } => VNodeResolved::Component(parent.child(key).expect("VNode::resolve failed: component not in parent")),
            VNode::View(view) => VNodeResolved::View(view)
        }
    }

    pub fn resolve_mut<'c, 'v>(&'v mut self, parent: &'c mut Box<VComponent<ViewData>>) -> VNodeResolvedMut<'c, 'v, ViewData> {
        match self {
            VNode::Component { id: _id, key } => VNodeResolvedMut::Component(parent.child_mut(key).expect("VNode::resolve failed: component not in parent")),
            VNode::View(view) => VNodeResolvedMut::View(view)
        }
    }

    pub fn update(&mut self, parent: &mut Box<VComponent<ViewData>>, details: Cow<'static, str>) {
        match self.resolve_mut(parent) {
            VNodeResolvedMut::Component(component) => {
                component.update(details);
            },
            VNodeResolvedMut::View(view) => {
                if let Some((children, _)) = view.d.children_mut() {
                    for (index, child) in children.enumerate() {
                        let sub_details = Cow::Owned(format!("{}[{}]", details, index));
                        child.update(parent, sub_details);
                    }
                }
            }
        }
    }

    pub fn component_and_view<'a>(&'a self, parent: &'a Box<VComponent<ViewData>>) -> VComponentAndView<'a, ViewData> {
        match self.resolve(parent) {
            VNodeResolved::Component(component) => component.component_and_view(),
            VNodeResolved::View(view) => (parent, view)
        }
    }

    pub fn view<'a>(&'a self, parent: &'a Box<VComponent<ViewData>>) -> &'a Box<VView<ViewData>> {
        match self.resolve(parent) {
            VNodeResolved::Component(component) => component.view(),
            VNodeResolved::View(view) => view
        }
    }
}

/* impl <'c, 'v, ViewData: VViewData> VNodeResolved<'c, 'v, ViewData> {
    pub fn view(&self) -> &VComponentAndView<ViewData> {
        match self {
            VNodeResolved::Component(component) => component.view(),
            VNodeResolved::View(view) => view
        }
    }
} */