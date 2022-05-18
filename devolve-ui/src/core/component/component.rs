use crate::core::component::context::VContext;
use crate::core::component::mode::VMode;
use crate::core::component::node::VNode;
use crate::renderer::Renderer;
use std::any::Any;
use std::borrow::Cow;
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use replace_with::replace_with_or_abort;

pub trait VComponentConstruct {
    fn construct(&self) -> VNode;
}

struct VComponentConstructImpl<Props: 'static, F: Fn(&Props) -> VNode + 'static> {
    props: Props,
    construct: F
}

pub type VComponentKey = Cow<'static, str>;

pub struct VComponent {
    /*readonly*/ id: usize,
    /*readonly*/ key: VComponentKey,

    construct: Box<dyn VComponentConstruct>,
    node: Option<VNode>,
    state: Vec<Box<dyn Any>>,
    // pub providedContexts: HashMap<Context, Box<dyn Any>>,
    // pub consumedContexts: HashMap<Context, Box<dyn Any>>
    effects: Vec<Box<dyn Fn(&mut Box<VComponent>) -> ()>>,
    update_destructors: Vec<Box<dyn FnOnce(&mut Box<VComponent>) -> ()>>,
    next_update_destructors: Vec<Box<dyn FnOnce(&mut Box<VComponent>) -> ()>>,
    permanent_destructors: Vec<Box<dyn FnOnce(&mut Box<VComponent>) -> ()>>,

    /*readonly*/ children: HashMap<VComponentKey, Box<VComponent>>,
    /*readonly*/ renderer: Weak<Renderer>,

    is_being_updated: bool,
    is_fresh: bool,
    is_dead: bool,
    has_pending_updates: bool,
    recursive_update_stack_trace: Vec<Cow<'static, str>>,
    next_state_index: usize
}

impl VComponent {
    pub fn root(renderer: Rc<Renderer>, construct: impl FnOnce() -> Box<Self>) {
        VContext::with_clear_component_stack(|| {
            let root_component = VContext::with_push_renderer(Rc::downgrade(&renderer), construct);
            renderer.set_root_component(Some(root_component));
        })
    }

    pub fn new<Props: 'static, F: Fn(&Props) -> VNode + 'static>(key: &VComponentKey, props: Props, construct: F) -> Box<Self> {
        enum Action<Props_, F_> {
            Reuse(Box<VComponent>),
            Create(Props_, F_)
        }

        let action = VContext::with_try_top_component(|parent| {
            if let Some(mut parent) = parent {
                // parent is being created = if there are any existing children, they're not being reused, they're a conflict
                if parent.node.is_some() {
                    let found_child = parent.children.remove(key);
                    if let Some(mut found_child) = found_child {
                        if found_child.is_fresh {
                            // If the component was already reused this update, it's a conflict. We add back and fall through to VComponent.create which will panic
                            parent.children.insert(key.clone(), found_child);
                        } else {
                            // Reuse child
                            found_child.construct = Box::new(VComponentConstructImpl {
                                props,
                                construct
                            });
                            found_child.is_fresh = true;
                            let details = Cow::Owned(format!("child:{}", key));
                            found_child.update(details);
                            // When we return this it will be added back to parent.children
                            return Action::Reuse(found_child)
                        }
                    }
                }
            }
            // Fallthrough case
            return Action::Create(props, construct)
        });
        match action {
            Action::Reuse(found_child) => found_child,
            Action::Create(props, construct) => Self::create(key, props, construct)
        }
    }

    fn create<Props: 'static, F: Fn(&Props) -> VNode + 'static>(key: &VComponentKey, props: Props, construct: F)  -> Box<Self>{
        Box::new(VComponent {
            id: VNode::next_id(),
            key: key.clone(),

            construct: Box::new(VComponentConstructImpl {
                props,
                construct
            }),
            node: None,
            state: Vec::new(),
            // providedContexts: HashMap::new(),
            // consumedContexts: HashMap::new(),
            effects: Vec::new(),
            update_destructors: Vec::new(),
            next_update_destructors: Vec::new(),
            permanent_destructors: Vec::new(),

            children: HashMap::new(),
            renderer: Rc::downgrade(&VContext::get_renderer()),

            is_being_updated: false,
            is_fresh: true,
            is_dead: false,
            has_pending_updates: false,
            recursive_update_stack_trace: Vec::new(),
            next_state_index: 0
        })
    }

    pub(crate) fn update(mut self: &mut Box<Self>, details: Cow<'static, str>) {
        if self.is_being_updated {
            // Delay until after this update, especially if there are multiple triggered updates since we only have to update once more
            self.has_pending_updates = true;
            if VMode::is_debug() {
                self.recursive_update_stack_trace.push(details)
            }
        } else if self.node.is_none() {
            // Do construct
            let details = Cow::Owned(format!("{}!", details));
            let child_details = Cow::Owned(format!("{}/", details));
            self.do_update(details, || {
                // Actually do construct and set node
                // This needs to be unsafe because code in self_.construct.construct()
                // needs implicit access to self through the context,
                // so we can't trigger borrow_mut because it will conflict.
                // This is safe because once we call self_.construct.construct we effectively
                // no longer need access to self_, even though there's no clear way to get
                // this through the type system.
                let mut node = unsafe {
                    VContext::with_top_component_unsafe(|self_| {
                        self_.construct.construct()
                    })
                };
                // from devolve-ui.js: "Create pixi if pixi component and on web"
                // should have a hook in view trait we can call through node.view().hook(...)

                // Update children (if box or another component)
                node.update(child_details);

                VContext::with_top_component(|mut self_| {
                    self_.node = Some(node)
                })
            })
        } else {
            // Reset
            self.run_update_destructors();
            self.next_state_index = 0;
            // self.provided_contexts.clear();

            // Do construct
            let child_details = Cow::Owned(format!("{}/", details));
            self.do_update(details, || {
                // This needs to be unsafe because code in self_.construct.construct()
                // needs implicit access to self through the context,
                // so we can't trigger borrow_mut because it will conflict.
                // This is safe because once we call self_.construct.construct we effectively
                // no longer need access to self_, even though there's no clear way to get
                // this through the type system.
                let mut node = unsafe {
                    VContext::with_top_component_unsafe(|self_| {
                        self_.construct.construct()
                    })
                };

                // from devolve-ui.js: "Update pixi if pixi component and on web"
                // should have a hook in view trait we can call through node.view().hook(...)

                // Update children (if box or another component)
                node.update(child_details);

                VContext::with_top_component(|mut self_| {
                    self_.invalidate();
                    self_.node = Some(node)
                })
            })
        }
    }

    fn destroy(mut self: Box<Self>) {
        assert!(self.node.is_some(), "tried to destroy uninitialized component");

        self.run_permanent_destructors();

        // from devolve-ui.js: "Destroy pixi if pixi component and on web"
        // should have a hook in view trait we can call through node.view().hook(...)

        self.invalidate();

        for child in self.children.into_values() {
            child.destroy()
        }
    }

    fn do_update(mut self: &mut Box<Self>, details: Cow<'_, str>, body: impl FnOnce() -> ()) {
        replace_with_or_abort(self, |self_| {
            VContext::with_push_renderer(self_.renderer.clone(), || {
                let ((), self_) = VContext::with_push_component(self_, || {
                    VContext::with_top_component(|mut self_| {
                        self_.is_being_updated = true;
                    });

                    body();

                    VContext::with_top_component(|mut self_| {
                        self_.clear_fresh_and_remove_stale_children();
                        self_.is_being_updated = false;
                        self_.run_effects()
                    })
                });
                self_
            })
        });

        if self.has_pending_updates {
            self.has_pending_updates = false;
            let recursive_update_stack_trace = &self.recursive_update_stack_trace;
            assert!(recursive_update_stack_trace.len() < VMode::max_recursive_updates_before_loop_detected(), "update loop detected:\n{}", recursive_update_stack_trace.join("\n"));
            let details = Cow::Owned(format!("{}^", details));
            self.update(details);
        } else {
            self.recursive_update_stack_trace.clear();
        }
    }

    fn clear_fresh_and_remove_stale_children(self: &mut Box<Self>) {
        let child_keys: Vec<VComponentKey> = self.children.keys().cloned().collect();
        for child_key in child_keys {
            let child = self.children.get_mut(&child_key).unwrap();
            if child.is_fresh {
                child.is_fresh = false
            } else {
                let child = self.children.remove(&child_key).unwrap();
                child.destroy();
            }
        }
    }

    fn run_effects(self: &mut Box<Self>) {
        while let Some(effect) = self.effects.pop() {
            if self.has_pending_updates {
                break
            }

            effect(self);
        }
    }

    fn run_update_destructors(self: &mut Box<Self>) {
        while let Some(update_destructor) = self.update_destructors.pop() {
            update_destructor(self)
        }
        self.update_destructors.append(&mut self.next_update_destructors);
    }

    fn run_permanent_destructors(self: &mut Box<Self>) {
        while let Some(permanent_destructor) = self.permanent_destructors.pop() {
            permanent_destructor(self)
        }
    }

    fn invalidate(self: &Box<Self>) {
        if let Some(renderer) = self.renderer.upgrade() {
            renderer.invalidate(self);
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn key(&self) -> VComponentKey {
        self.key.clone()
    }

    pub fn node(&self) -> Option<&VNode> {
        self.node.as_ref()
    }
}

impl <Props: 'static, F: Fn(&Props) -> VNode + 'static> VComponentConstruct for VComponentConstructImpl<Props, F> {
    fn construct(&self) -> VNode {
        let construct = &self.construct;
        construct(&self.props)
    }
}