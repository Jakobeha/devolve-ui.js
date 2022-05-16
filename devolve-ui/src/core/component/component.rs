use crate::core::component::node::VNode;
use std::any::Any;
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use crate::renderer::Renderer;

pub trait VComponentConstruct {
    type Props;

    fn props(&self) -> &Self::Props;
    fn props_mut(&mut self) -> &mut Self::Props;
    fn _construct(props: &Self::Props) -> VNode;

    fn construct(&self) -> VNode {
        Self::_construct(self.props())
    }
}

pub struct VComponent {
    pub key: String,

    construct: Box<dyn VComponentConstruct>,
    pub node: Option<VNode>,
    state: Vec<Box<dyn Any>>,
    // pub providedContexts: HashMap<Context, Box<dyn Any>>,
    // pub consumedContexts: HashMap<Context, Box<dyn Any>>
    effects: Vec<Box<dyn FnOnce() -> ()>>,
    update_destructors: Vec<Box<dyn FnOnce() -> ()>>,
    next_update_destructors: Vec<Box<dyn FnOnce() -> ()>>,
    permanent_destructors: Vec<Box<dyn FnOnce() -> ()>>,

    children: HashMap<str, Rc<VComponent>>,
    renderer: Weak<Renderer>,

    is_being_updated: bool,
    is_fresh: bool,
    is_dead: bool,
    has_pending_updates: bool,
    recursive_update_stack_trace: Vec<String>,
    next_state_index: usize
}

impl VComponent {

}