use crate::core::component::component::VComponent;
use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;

pub struct Renderer {
    root_component: RefCell<Option<Box<VComponent>>>
}

impl Renderer {
    pub fn set_root_component(self: Rc<Renderer>, root_component: Option<Box<VComponent>>) {
        let mut self_root_component = self.root_component.borrow_mut();
        *self_root_component = root_component;
        if let Some(self_root_component) = self_root_component.as_mut() {
            self_root_component.update(Cow::Borrowed("init:"));
        }
    }
}