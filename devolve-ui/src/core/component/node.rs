use std::borrow::Cow;
use crate::core::component::component::VComponent;
use crate::core::view::view::VView;

pub enum VNode {
    Component(Box<VComponent>),
    View(Box<VView>)
}

static mut NEXT_ID: usize = 0;

impl VNode {
    pub fn next_id() -> usize {
        // TODO: Make thread safe?
        let id: usize;
        unsafe {
            NEXT_ID += 1;
            id = NEXT_ID;
        }
        id
    }

    pub fn id(&self) -> usize {
        match self {
            VNode::Component(component) => component.id(),
            VNode::View(view) => view.id()
        }
    }

    pub fn update(&mut self, details: Cow<'_, str>) {
        match self {
            VNode::Component(component) => {
                component.update(details);
            },
            VNode::View(view) => {
                for (index, child) in view.children_mut().iter_mut().enumerate() {
                    let sub_details = Cow::Owned(format!("{}[{}]", details, index));
                    child.update(sub_details);
                }
            }
        }
    }

    pub fn view(&self) -> &VView {
        match self {
            VNode::Component(component) => component
                .node()
                .expect("tried to get view from uninitialized component")
                .view(),
            VNode::View(view) => view
        }
    }
}