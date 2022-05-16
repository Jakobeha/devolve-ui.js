use crate::core::component::node::VNode;
use std::any::Any;
use std::borrow::{BorrowMut, Cow};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use crate::core::component::context::VContext;
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

struct VComponentConstructImpl<F: Fn(Props) -> VNode, Props> {
    props: Props,
    construct: F
}

pub type VComponentKey = Cow<'_, str>;

pub struct VComponent {
    pub key: VComponentKey,

    construct: Box<dyn VComponentConstruct>,
    pub node: Option<VNode>,
    state: RefCell<Vec<Box<dyn Any>>>,
    // pub providedContexts: HashMap<Context, Box<dyn Any>>,
    // pub consumedContexts: HashMap<Context, Box<dyn Any>>
    effects: RefCell<Vec<Box<dyn Fn() -> ()>>>,
    update_destructors: RefCell<Vec<Box<dyn FnOnce() -> ()>>>,
    next_update_destructors: RefCell<Vec<Box<dyn FnOnce() -> ()>>>,
    permanent_destructors: RefCell<Vec<Box<dyn FnOnce() -> ()>>>,

    children: HashMap<VComponentKey, Rc<VComponent>>,
    renderer: Weak<Renderer>,

    is_being_updated: bool,
    is_fresh: bool,
    is_dead: bool,
    has_pending_updates: bool,
    recursive_update_stack_trace: RefCell<Vec<Cow<'_, str>>>,
    next_state_index: Cell<usize>
}

impl VComponent {
    pub fn root(renderer: Weak<Renderer>, construct: impl FnOnce() -> VComponent) -> VComponent {
        VContext::with_empty_component_stack(|| {
            let mut node = VContext::with_renderer(renderer, construct);
            node.update("init:");
            node
        })
    }

    pub fn new<Props, F: Fn(Props) -> VNode>(key: &VComponentKey, props: Props, construct: F) -> Rc<VComponent> {
        if let Some(parent) = VContext::try_get_component() {
            if (!parent.is_being_updated) {
                // TODO: try to reuse this component
            }
        }

        Self::create(key, props, construct)
    }

    fn create<Props, F: Fn(Props) -> VNode>(key: &VComponentKey, props: Props, construct: F) -> Rc<VComponent> {
        let component = Rc::new(VComponent {
            key: key.clone(),

            construct: Box::new(VComponentConstructImpl {
                props,
                construct
            }),
            node: None,
            state: RefCell::new(Vec::new()),
            // providedContexts: HashMap::new(),
            // consumedContexts: HashMap::new(),
            effects: RefCell::new(Vec::new()),
            update_destructors: RefCell::new(Vec::new()),
            next_update_destructors: RefCell::new(Vec::new()),
            permanent_destructors: RefCell::new(Vec::new()),

            children: HashMap::new(),
            renderer: Rc::downgrade(&VContext::get_renderer()),

            is_being_updated: false,
            is_fresh: true,
            is_dead: false,
            has_pending_updates: false,
            recursive_update_stack_trace: RefCell::new(Vec::new()),
            next_state_index: Cell::new(0)
        });

        if let Some(parent) = VContext::try_get_component() {
            let mut children = parent.children.borrow_mut();
            assert!(children.insert(key.clone(), component.clone()).is_none());
        } else {
            VContext::get_renderer().set_root_component(component.clone());
        }

        component
    }

    fn update(self: &mut Rc<Self>, details: Cow<'_, str>) {
        if self.is_being_updated {
            // Delay until after this update, especially if there are multiple triggered updates since we only have to update once more
            self.has_pending_updates = true;
            if VMode::is_debug() {
                self.recursive_update_stack_trace.push(details)
            }
        } else if self.node.is_none() {
            // Do construct
            let details = format!("{}!", details);
            let child_details = format!("{}/", details);
            self.do_update(Cow::Owned(details), || {
                // Actually do construct and set component.node
                let node = self.construct.construct();

                // from devolve-ui.js: "Create pixi if pixi component and on web"

                // Update children (if box or another component)
                node.update(child_details);
                self.node = Some(node)
            })
        } else {
            // Reset
            self.run_update_destructors();
            *self.next_state_index = 0;
            // self.provided_contexts.clear();

            // Do construct
            let child_details = format!("{}/", details);
            self.do_update(details, || {
                let node = self.construct.construct();

                // from devolve-ui.js: "Update pixi if pixi component and on web"

                // Update children (if box or another component)
                node.update(child_details);

                self.invalidate();
                self.node = Some(node);
            })
        }
    }

    fn destroy(self: Rc<Self>) {
        assert!(self.node.is_some(), "tried to destroy uninitialized component");

        self.run_permanent_destructors();

        // from devolve-ui.js: "Destroy pixi if pixi component and on web"

        self.invalidate();

        for child in self.children.into_values() {
            child.destroy()
        }
    }

    fn do_update(self: &mut Rc<Self>, details: Cow<'_, str>, body: impl FnOnce() -> ()) {
        VContext::with_renderer(self.renderer.clone(), VContext::with_component(Rc::downgrade(self), || {
            self.is_being_updated = true;

            body();

            self.clear_fresh_and_remove_stale_children();
            self.is_being_updated = false;
            self.run_effects();
        }));

        if self.has_pending_updates {
            self.has_pending_updates = false;
            let recursive_update_stack_trace = self.recursive_update_stack_trace.borrow();
            assert!(recursive_update_stack_trace.len() < VNode::max_recursive_updates_before_loop_detected(), "update loop detected:\n{}", recursive_update_stack_trace.join("\n"));
            let details = format!("{}^", details);
            self.update(Cow::Owned(details));
        } else {
            self.recursive_update_stack_trace.borrow_mut().clear();
        }
    }

    fn clear_fresh_and_remove_stale_children(self: &mut Rc<Self>) {
        for (child_key, child) in self.children.clone() {
            if child.is_fresh {
                child.is_fresh = false
            } else {
                child.destroy();
                self.children.remove(&*child_key)
            }
        }
    }
}