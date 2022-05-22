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

pub enum VNode<ViewData: VViewData> {
    Component(Box<VComponent<ViewData>>),
    View(Box<VView<ViewData>>)
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
            VNode::View(view) => view.id
        }
    }

    pub fn update(&mut self, details: Cow<'static, str>) {
        match self {
            VNode::Component(component) => {
                component.update(details);
            },
            VNode::View(view) => {
                if let Some((children, _)) = view.d.children_mut() {
                    for (index, child) in children.enumerate() {
                        let sub_details = Cow::Owned(format!("{}[{}]", details, index));
                        child.update(sub_details);
                    }
                }
            }
        }
    }

    pub fn view(&self) -> &Box<VView<ViewData>> {
        match self {
            VNode::Component(component) => component.view(),
            VNode::View(view) => view
        }
    }
}