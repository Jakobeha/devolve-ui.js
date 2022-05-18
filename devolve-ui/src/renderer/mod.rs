use crate::core::component::component::VComponent;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

struct ParentId(usize);

impl ParentId {
    const NO_PARENT: Self = Self(0);
}

struct VRender {
    layers: HashMap<f32, VRenderLayer>,
    rect: Option<Rectangle>
}

struct CachedRender {
    render: VRender,
    parent_bounds: ParentBounds,
    sibling_rect: Option<Rectangle>,
    parent: ParentId
}

pub struct Renderer {
    fps: u32,
    cached_renders: HashMap<usize, CachedRender>,
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

    pub fn invalidate(self: Rc<Renderer>, component: &Box<VComponent>) {
        self.invalidate(component.node().expect("invalidate called with uninitialized component"))
    }

    pub fn invalidate(self: Rc<Renderer>, node: &VNode) {

    }
}