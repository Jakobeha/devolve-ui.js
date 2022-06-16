//! A `VNode` is either a component of a view. Either way, the node contains content which is rendered,
//! and may contain child nodes.

use std::fmt;
use std::fmt::{Display, Formatter};
use crate::core::component::component::{VComponent, VComponentHead};
use crate::core::component::path::VComponentKey;
use crate::core::view::view::{VView, VViewData};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NodeId(usize);

impl Display for NodeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum VNode<ViewData: VViewData> {
    Component { id: NodeId, key: VComponentKey },
    View(Box<VView<ViewData>>)
}

#[derive(Debug)]
enum VNodeResolvedHead<'c, 'v, ViewData: VViewData> {
    Component(&'c VComponentHead<ViewData>),
    View(&'v Box<VView<ViewData>>)
}

// Lifetimes must be different for `VNode::update`
#[derive(Debug)]
enum VNodeResolvedMut<'c, 'v, ViewData: VViewData> {
    Component(&'c mut Box<VComponent<ViewData>>),
    View(&'v mut Box<VView<ViewData>>)
}

// Lifetimes don't need to be different here. Might want to rename this type
// (maybe to VViewRenderable or VView and then rename VView to VViewRaw or something).
// Basically we need the view's component when rendering it and using it in many other scenarios,
// because we need this component in order to resolve the view's children,
// since they may be child components and child components are stored in the component
// so they can be resolved even when a component has the same key but switches views.
pub type VComponentAndView<'a, ViewData> = (&'a VComponentHead<ViewData>, &'a Box<VView<ViewData>>);

static mut NEXT_ID: usize = 0;

impl NodeId {
    pub const NULL: NodeId = NodeId(0);

    pub fn next() -> NodeId {
        // TODO: Make thread safe?
        unsafe {
            NEXT_ID += 1;
            NodeId(NEXT_ID)
        }
    }
}

impl <ViewData: VViewData> VNode<ViewData> {
    pub fn id(&self) -> NodeId {
        match self {
            VNode::Component { id, key: _key} => *id,
            VNode::View(view) => view.id()
        }
    }

    fn resolve<'c, 'v>(&'v self, parent: &'c VComponentHead<ViewData>) -> VNodeResolvedHead<'c, 'v, ViewData> {
        match self {
            VNode::Component { id: _id, key } => VNodeResolvedHead::Component(parent.child(key).expect("VNode::resolve failed: component not in parent")),
            VNode::View(view) => VNodeResolvedHead::View(view)
        }
    }

    fn resolve_mut<'c, 'v>(&'v mut self, parent: &'c mut Box<VComponent<ViewData>>) -> VNodeResolvedMut<'c, 'v, ViewData> {
        match self {
            VNode::Component { id: _id, key } => VNodeResolvedMut::Component(parent.child_mut(key).expect("VNode::resolve failed: component not in parent")),
            VNode::View(view) => VNodeResolvedMut::View(view)
        }
    }

    // Lifetimes must be different here because we borrow parent in resolve_mut and in the VNodeResolvedMut::View case.
    // This is OK because the lifetime in VNodeResolvedMut::View's view is different than that in parent
    pub fn update(&mut self, parent: &mut Box<VComponent<ViewData>>) {
        match self.resolve_mut(parent) {
            VNodeResolvedMut::Component(component) => {
                component.update();
            },
            VNodeResolvedMut::View(view) => {
                if let Some((children, _)) = view.d.children_mut() {
                    for child in children {
                        child.update(parent);
                    }
                }
            }
        }
    }

    pub fn component_and_view<'a>(&'a self, parent: &'a VComponentHead<ViewData>) -> VComponentAndView<'a, ViewData> {
        match self.resolve(parent) {
            VNodeResolvedHead::Component(component) => component.component_and_view(),
            VNodeResolvedHead::View(view) => (parent, view)
        }
    }

    pub fn view<'a>(&'a self, parent: &'a VComponentHead<ViewData>) -> &'a Box<VView<ViewData>> {
        match self.resolve(parent) {
            VNodeResolvedHead::Component(component) => component.view(),
            VNodeResolvedHead::View(view) => view
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