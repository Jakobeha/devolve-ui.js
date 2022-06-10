//! A `VNode` is either a component of a view. Either way, the node contains content which is rendered,
//! and may contain child nodes.

use std::borrow::Cow;
use std::fmt;
use std::fmt::{Display, Formatter};
use crate::core::component::component::VComponent;
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
    Component(Box<VComponent<ViewData>>),
    View(Box<VView<ViewData>>)
}

pub enum VNodeRef<'a, ViewData: VViewData> {
    Component(&'a Box<VComponent<ViewData>>),
    View(&'a Box<VView<ViewData>>)
}

pub enum VNodeMut<'a, ViewData: VViewData> {
    Component(&'a mut Box<VComponent<ViewData>>),
    View(&'a mut Box<VView<ViewData>>)
}

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
            VNode::Component(component) => component.id(),
            VNode::View(view) => view.id()
        }
    }

    pub fn view(&self) -> &Box<VView<ViewData>> {
        match self {
            VNode::Component(component) => component.view(),
            VNode::View(view) => view
        }
    }

    pub fn update(&mut self, details: Cow<'static, str>) {
        self.as_mut().update(details)
    }

    pub fn as_ref(&self) -> VNodeRef<'_, ViewData> {
        match self {
            VNode::Component(component) => VNodeRef::Component(component),
            VNode::View(view) => VNodeRef::View(view)
        }
    }

    pub fn as_mut(&mut self) -> VNodeMut<'_, ViewData> {
        match self {
            VNode::Component(component) => VNodeMut::Component(component),
            VNode::View(view) => VNodeMut::View(view)
        }
    }
}

impl <'a, ViewData: VViewData> VNodeRef<'a, ViewData> {
    pub fn id(&self) -> NodeId {
        match self {
            VNodeRef::Component(component) => component.id(),
            VNodeRef::View(view) => view.id()
        }
    }

    pub fn view(&self) -> &Box<VView<ViewData>> {
        match self {
            VNodeRef::Component(component) => component.view(),
            VNodeRef::View(view) => view
        }
    }

    pub fn into_component(self) -> Option<&'a Box<VComponent<ViewData>>> {
        match self {
            VNodeRef::Component(component) => Some(component),
            _ => None
        }
    }
}

impl <'a, ViewData: VViewData> VNodeMut<'a, ViewData> {
    pub fn update(&mut self, details: Cow<'static, str>) {
        match self {
            VNodeMut::Component(component) => {
                component.update(details);
            },
            VNodeMut::View(view) => {
                if let Some((children, _)) = view.d.children_mut() {
                    for (index, child) in children.enumerate() {
                        let sub_details = Cow::Owned(format!("{}[{}]", details, index));
                        child.update(sub_details);
                    }
                }
            }
        }
    }

    pub fn into_component(self) -> Option<&'a mut Box<VComponent<ViewData>>> {
        match self {
            VNodeMut::Component(component) => Some(component),
            _ => None
        }
    }
}