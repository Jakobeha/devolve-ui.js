///! A `VNode` is either a component of a view. Either way, the node contains content which is rendered,
/// and may contain child nodes.

use std::borrow::Cow;
use std::fmt;
use std::fmt::{Display, Formatter};
use crate::core::component::component::VComponent;
use crate::core::component::path::{VNodeKey, VNodePath, VNodePathSegment};
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

    pub fn key(&self) -> VNodeKey {
        match self {
            VNode::Component(component) => component.key(),
            VNode::View(view) => view.key()
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

    pub fn down_path<'a>(&'a self, path: &'a VNodePath) -> Option<VNodeRef<'_, ViewData>> {
        self.as_ref().down_path(path)
    }

    pub fn down_path_mut<'a>(&'a mut self, path: &'a VNodePath) -> Option<VNodeMut<'_, ViewData>> {
        self.as_mut().down_path_mut(path)
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

    pub fn key(&self) -> VNodeKey {
        match self {
            VNodeRef::Component(component) => component.key(),
            VNodeRef::View(view) => view.key()
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

    pub fn down_path(self, path: &'a VNodePath) -> Option<VNodeRef<'_, ViewData>> {
        let mut current = self;
        for elem in path.iter() {
            current = current.down_path_segment(elem)?;
        }
        Some(current)
    }

    fn down_path_segment(self, segment: &'a VNodePathSegment) -> Option<VNodeRef<'_, ViewData>> {
        match (self, segment) {
            (VNodeRef::Component(component), VNodePathSegment::ComponentChild) => {
                component.node_ref()
            }
            (VNodeRef::View(view), VNodePathSegment::ViewChildWithKey(key)) => {
                view.d.children().and_then(|(mut children, _)| {
                    children.find(|child| &child.key() == key).map(|child| child.as_ref())
                })
            }
            (VNodeRef::View(view), VNodePathSegment::ViewChildWithIndex(index)) => {
                view.d.children().and_then(|(mut children, _)| {
                    children.nth(*index).map(|child| child.as_ref())
                })
            }
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

    pub fn down_path_mut(self, path: &'a VNodePath) -> Option<VNodeMut<'_, ViewData>> {
        let mut current = self;
        for elem in path.iter() {
            current = current.down_path_segment_mut(elem)?;
        }
        Some(current)
    }

    fn down_path_segment_mut(self, segment: &'a VNodePathSegment) -> Option<VNodeMut<'_, ViewData>> {
        match (self, segment) {
            (VNodeMut::Component(component), VNodePathSegment::ComponentChild) => {
                component.node_mut()
            }
            (VNodeMut::View(view), VNodePathSegment::ViewChildWithKey(key)) => {
                view.d.children_mut().and_then(|(mut children, _)| {
                    children.find(|child| &child.key() == key).map(|child| child.as_mut())
                })
            }
            (VNodeMut::View(view), VNodePathSegment::ViewChildWithIndex(index)) => {
                view.d.children_mut().and_then(|(mut children, _)| {
                    children.nth(*index).map(|child| child.as_mut())
                })
            }
            _ => None
        }
    }
}