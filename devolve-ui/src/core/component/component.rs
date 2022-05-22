use crate::core::component::context::VParent;
use crate::core::component::mode::VMode;
use crate::core::component::node::{NodeId, VNode};
use std::any::Any;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::{Rc, Weak};
use crate::core::view::view::{VView, VViewData};

pub(in core) trait VComponentRoot {
    type ViewData: VViewData;

    fn invalidate(self: Rc<Self>, view: &Box<VView<Self::ViewData>>);
}

pub trait VComponentConstruct {
    type ViewData: VViewData;

    fn construct(&self, component: &mut Box<VComponent<Self::ViewData>>) -> VNode<Self::ViewData>;
}

struct VComponentConstructImpl<ViewData: VViewData, Props: 'static, F: Fn(&mut Box<VComponent<ViewData>>, &Props) -> VNode<ViewData> + 'static> {
    props: Props,
    construct: F,
    view_data_type: PhantomData<ViewData>
}

pub type VComponentKey = Cow<'static, str>;

pub struct VComponent<ViewData: VViewData> {
    /*readonly*/ id: NodeId,
    /*readonly*/ key: VComponentKey,

    construct: Box<dyn VComponentConstruct<ViewData = ViewData>>,
    node: Option<VNode<ViewData>>,
    pub(in crate::core) state: Vec<Box<dyn Any>>,
    // pub(in crate::core::hooks) providedContexts: HashMap<Context, Box<dyn Any>>,
    // pub(in crate::core::hooks) consumedContexts: HashMap<Context, Box<dyn Any>>
    pub(in crate::core) effects: Vec<Box<dyn Fn(&mut Box<VComponent<ViewData>>) -> ()>>,
    pub(in crate::core) update_destructors: Vec<Box<dyn FnOnce(&mut Box<VComponent<ViewData>>) -> ()>>,
    pub(in crate::core) next_update_destructors: Vec<Box<dyn FnOnce(&mut Box<VComponent<ViewData>>) -> ()>>,
    pub(in crate::core) permanent_destructors: Vec<Box<dyn FnOnce(&mut Box<VComponent<ViewData>>) -> ()>>,

    /*readonly*/ children: HashMap<VComponentKey, Box<VComponent<ViewData>>>,
    /*readonly*/ renderer: Weak<dyn VComponentRoot<ViewData = ViewData>>,

    is_being_updated: bool,
    is_fresh: bool,
    has_pending_updates: bool,
    recursive_update_stack_trace: Vec<Cow<'static, str>>,
    pub(in crate::core) next_state_index: usize
}

impl <ViewData: VViewData + 'static> VComponent<ViewData> {
    pub fn new<Props: 'static, F: Fn(&mut Box<VComponent<ViewData>>, &Props) -> VNode<ViewData> + 'static>(parent: VParent<'_, ViewData>, key: &VComponentKey, props: Props, construct: F) -> Box<Self> {
        enum Action<'a, ViewData_: VViewData, Props_, F_> {
            Reuse(Box<VComponent<ViewData_>>),
            Create(VParent<'a, ViewData_>, Props_, F_)
        }

        let action = (|| {
            if let VParent::Component(parent) = parent {
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
                                construct,
                                view_data_type: PhantomData,
                            });
                            found_child.is_fresh = true;
                            let details = Cow::Owned(format!("child:{}", key));
                            found_child.update(details);
                            // When we return this it will be added back to parent.children
                            return Action::Reuse(found_child)
                        }
                    }
                }
                // Fallthrough case (undo move)
                let parent = VParent::Component(parent);
                return Action::Create(parent, props, construct)
            }
            // Fallthrough case
            return Action::Create(parent, props, construct)
        })();
        match action {
            Action::Reuse(found_child) => found_child,
            Action::Create(parent, props, construct) => Self::create(parent, key, props, construct)
        }
    }

    fn create<Props: 'static, F: Fn(&mut Box<VComponent<ViewData>>, &Props) -> VNode<ViewData> + 'static>(parent: VParent<'_, ViewData>, key: &VComponentKey, props: Props, construct: F)  -> Box<Self>{
        Box::new(VComponent {
            id: VNode::<ViewData>::next_id(),
            key: key.clone(),

            construct: Box::new(VComponentConstructImpl {
                props,
                construct,
                view_data_type: PhantomData
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
            renderer: match parent {
                VParent::Root(renderer) => Rc::downgrade(renderer),
                VParent::Component(component) => component.renderer.clone()
            },

            is_being_updated: false,
            is_fresh: true,
            has_pending_updates: false,
            recursive_update_stack_trace: Vec::new(),
            next_state_index: 0
        })
    }
}

impl <ViewData: VViewData> VComponent<ViewData> {
    pub(in core) fn update(mut self: &mut Box<Self>, details: Cow<'static, str>) {
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
            self.do_update(details, |self_| {
                // Actually do construct and set node
                // This is safe because we only borrow the field 'construct', and the function 'construct.construct' does not borrow the field 'construct'
                let construct = unsafe { (&self_.construct as *const Box<dyn VComponentConstruct<ViewData = ViewData>>).as_ref().unwrap() };
                let mut node = construct.construct(self_);

                // from devolve-ui.js: "Create pixi if pixi component and on web"
                // should have a hook in view trait we can call through node.view().hook(...)

                // Update children (if box or another component)
                node.update(child_details);

                self_.node = Some(node)
            })
        } else {
            // Reset
            self.run_update_destructors();
            self.next_state_index = 0;
            // self.provided_contexts.clear();

            // Do construct
            let child_details = Cow::Owned(format!("{}/", details));
            self.do_update(details, |self_| {
                // This is safe because we only borrow the field 'construct', and the function 'construct.construct' does not borrow the field 'construct'
                let construct = unsafe { (&self_.construct as *const Box<dyn VComponentConstruct<ViewData = ViewData>>).as_ref().unwrap() };
                let mut node = construct.construct(self_);

                // from devolve-ui.js: "Update pixi if pixi component and on web"
                // should have a hook in view trait we can call through node.view().hook(...)

                // Update children (if box or another component)
                node.update(child_details);

                self_.invalidate();
                self_.node = Some(node)
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

    fn do_update(mut self: &mut Box<Self>, details: Cow<'_, str>, body: impl FnOnce(&mut Box<Self>) -> ()) {
        self.is_being_updated = true;

        body(self);

        self.clear_fresh_and_remove_stale_children();
        self.is_being_updated = false;
        self.run_effects();

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
            renderer.invalidate(self.view());
        }
    }

    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn key(&self) -> VComponentKey {
        self.key.clone()
    }

    pub fn is_being_created(&self) -> bool {
        self.node.is_none()
    }

    pub fn view(&self) -> &Box<VView<ViewData>> {
        self.node.as_ref().expect("tried to get view of uninitialized component").view()
    }
}

impl <ViewData: VViewData, Props: 'static, F: Fn(&mut Box<VComponent<ViewData>>, &Props) -> VNode<ViewData> + 'static> VComponentConstruct for VComponentConstructImpl<ViewData, Props, F> {
    type ViewData = ViewData;

    fn construct(&self, component: &mut Box<VComponent<Self::ViewData>>) -> VNode<Self::ViewData> {
        let construct = &self.construct;
        construct(component, &self.props)
    }
}