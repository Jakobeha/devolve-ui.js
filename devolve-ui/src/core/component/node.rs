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

pub enum VNode<'a, ViewData: VViewData<'a>> {
    Component(Box<VComponent<'a, ViewData>>),
    View(Box<VView<'a, ViewData>>)
}

static mut NEXT_ID: usize = 0;

impl <'a, ViewData: VViewData<'a>> VNode<'a, ViewData> {
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
            VNode::View(view) => view.id
        }
    }

    pub fn update(&mut self, details: Cow<'static, str>) {
        match self {
            VNode::Component(component) => {
                component.update(details);
            },
            VNode::View(view) => {
                for (index, child) in view.children_mut().enumerate() {
                    let sub_details = Cow::Owned(format!("{}[{}]", details, index));
                    child.update(sub_details);
                }
            }
        }
    }

    pub fn view(&self) -> &Box<VView<ViewData>> {
        match self {
            VNode::Component(component) => component
                .node()
                .expect("tried to get view from uninitialized component")
                .view(),
            VNode::View(view) => view
        }
    }
}