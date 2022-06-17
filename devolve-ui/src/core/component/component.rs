//! Components encapsulate UI elements and their behavior.
//! They take primitive views like text and borders, and with access to state and effects,
//! turn them into something meaningful like controls, data sections, prompts, or your entire app.
//!
//! Each component is defined by a function, which takes the component's "props" (input)
//! and returns a node which is the "render" of the component. This render is derived from the input props
//! and the component's state. Additionally, it may contain `hooks` like `use_state` to define state,
//! `use_effect` to run side-effects at certain points in the component's lifecycle, and other `use_`
//! hooks for events like time passing and keyboard / mouse input.
//! The function will be called every time the component re-renders.
//! See ["components and props" in the React docs](https://reactjs.org/docs/components-and-props.html)
//! for more info.
//!
//! Components are defined via the `make_component` macro.
//! Components can then be created by calling the function (verbose but "real Rust")
//! or macro (shorthand) defined by the `make_component` macro's expansion.
//!
//! Each component must have a `key` to distinguish it between updates,
//! but this key can be `""` or `()` if it's the only child. The key is provided upon creation.

use crate::core::component::parent::VParent;
use crate::core::component::mode::VMode;
use crate::core::component::node::{NodeId, VComponentAndView, VNode};
use std::any::Any;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::{Rc, Weak};
use std::fmt::{Debug, Formatter};
use crate::core::component::context::{VComponentContext1, VComponentContext2, VDestructorContext1, VDestructorContext2, VEffectContext1, VEffectContext2};
use crate::core::component::path::{VComponentKey, VComponentPath, VComponentRef};
use crate::core::component::root::VComponentRoot;
use crate::core::component::update_details::{UpdateDetails, UpdateStack};
#[cfg(feature = "logging")]
use crate::core::logging::update_logger::{UpdateLogEntry, UpdateLogger};
use crate::core::renderer::stale_data::NeedsUpdateFlag;
use crate::core::view::view::{VView, VViewData};

// region VComponent and sub-structures
/// You don't usually use this directly
pub struct VComponent<ViewData: VViewData> {
    /// Part of component which doesn't depend on `Props`.
    pub head: VComponentHead<ViewData>,
    /// Part of component with data whose size depends on `Props`, so it's runtime-sized.
    pub(super) construct: Box<dyn VComponentConstruct<ViewData = ViewData>>,
}

/// Part of component which doesn't depend on `Props`.
/// You don't usually call methods on this directly but instead pass it to hooks and other constructors.
pub struct VComponentHead<ViewData: VViewData> {
    /* readonly pub */  id: NodeId,
    /* readonly pub */  key: VComponentKey,
    /* readonly pub */  parent_path: VComponentPath,

    node: Option<VNode<ViewData>>,

    /*readonly*/ children: HashMap<VComponentKey, Box<VComponent<ViewData>>>,
    /*readonly*/ renderer: Weak<dyn VComponentRoot<ViewData = ViewData>>,

    pub(in crate::core) h: VComponentStateData,

    is_being_updated: bool,
    is_fresh: bool,
    has_pending_updates: bool,
    recursive_update_stack_trace: UpdateStack,
}

pub(in crate::core) struct VComponentStateData {
    pub state: Vec<Box<dyn Any>>,
    pub next_state_index: usize,
    // pub(in crate::core) provided_contexts: HashMap<Context, Box<dyn Any>>,
    // pub(in crate::core) consumed_contexts: HashMap<Context, Box<dyn Any>>
}

/// Part of the component with data whose size depends on `Props`, so it's a runtime-sized trait object.
pub(super) trait VComponentConstruct: Debug {
    type ViewData: VViewData;

    fn props(&self) -> &dyn Any;
    fn construct(&mut self, component: &mut VComponentHead<Self::ViewData>) -> VNode<Self::ViewData>;

    fn run_effects(&mut self, component: &mut VComponentHead<Self::ViewData>);
    fn run_update_destructors(&mut self, component: &mut VComponentHead<Self::ViewData>);
    fn run_permanent_destructors(&mut self, component: &mut VComponentHead<Self::ViewData>);
}

/// Part of the component with data whose size depends on `Props`. This is the compile-time sized implementation
/// which requires a specific `Props`.
struct VComponentConstructImpl<Props: Any, ViewData: VViewData, F: Fn(VComponentContext2<'_, Props, ViewData>) -> VNode<ViewData> + 'static> {
    props: Props,
    construct: F,
    effects: VComponentEffects<Props, ViewData>,
    destructors: VComponentDestructors<Props, ViewData>,
    view_data_type: PhantomData<ViewData>
}

pub(in crate::core) struct VComponentEffects<Props: Any, ViewData: VViewData> {
    pub effects: Vec<Box<dyn Fn(VEffectContext2<'_, Props, ViewData>) -> ()>>,
}

pub(in crate::core) struct VComponentDestructors<Props: Any, ViewData: VViewData> {
    pub update_destructors: Vec<Box<dyn FnOnce(VDestructorContext2<'_, Props, ViewData>) -> ()>>,
    pub next_update_destructors: Vec<Box<dyn FnOnce(VDestructorContext2<'_, Props, ViewData>) -> ()>>,
    pub permanent_destructors: Vec<Box<dyn FnOnce(VDestructorContext2<'_, Props, ViewData>) -> ()>>
}
// endregion

// region impls
impl <ViewData: VViewData + 'static> VComponent<ViewData> {
    /// Create a new component *or* update and reuse the existing component, if it has the same parent or key.
    pub(in crate::core) fn new<Props: 'static, F: Fn(VComponentContext2<'_, Props, ViewData>) -> VNode<ViewData> + 'static>(parent: VParent<'_, ViewData>, key: VComponentKey, props: Props, construct: F) -> Box<VComponent<ViewData>> {
        enum Action<'a, ViewData_: VViewData, Props_, F_> {
            Reuse(Box<VComponent<ViewData_>>),
            Create(VParent<'a, ViewData_>, Props_, F_)
        }

        let action = (|| {
            if let VParent::Component(parent) = parent {
                // parent is being created = if there are any existing children, they're not being reused, they're a conflict
                if parent.node.is_some() {
                    let found_child = parent.children.remove(&key);
                    if let Some(mut found_child) = found_child {
                        if found_child.head.is_fresh {
                            // If the component was already reused this update, it's a conflict. We add back and fall through to VComponent.create which will panic
                            parent.children.insert(key.clone(), found_child);
                        } else {
                            // Reuse child
                            found_child.construct = Box::new(VComponentConstructImpl {
                                props,
                                construct,
                                effects: VComponentEffects::new(),
                                destructors: VComponentDestructors::new(),
                                view_data_type: PhantomData,
                            });
                            found_child.head.is_fresh = true;
                            found_child.update();
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

    /// Create a new component with the parent and key. Don't call if there is already an existing component.
    fn create<Props: 'static, F: Fn(VComponentContext2<'_, Props, ViewData>) -> VNode<ViewData> + 'static>(parent: VParent<'_, ViewData>, key: VComponentKey, props: Props, construct: F) -> Box<Self>{
        Box::new(VComponent {
            head: VComponentHead {
                id: NodeId::next(),
                key,
                parent_path: parent.path(),
                node: None,
                children: HashMap::new(),
                renderer: match parent {
                    VParent::Root(renderer) => Rc::downgrade(renderer),
                    VParent::Component(component) => component.renderer.clone()
                },

                h: VComponentStateData {
                    state: Vec::new(),
                    next_state_index: 0,
                    // provided_contexts: HashMap::new(),
                    // consumed_contexts: HashMap::new(),
                },

                is_being_updated: false,
                is_fresh: true,
                // Create = needs update
                has_pending_updates: true,
                recursive_update_stack_trace: UpdateStack::new(),
            },

            construct: Box::new(VComponentConstructImpl {
                props,
                construct,
                effects: VComponentEffects::new(),
                destructors: VComponentDestructors::new(),
                view_data_type: PhantomData
            }),
        })
    }
}

impl <ViewData: VViewData> VComponent<ViewData> {
    /// Run pending updates on this component.
    pub(in crate::core) fn update(mut self: &mut Box<Self>) {
        while self.head.has_pending_updates {
            self.head.has_pending_updates = false;
            self.head.recursive_update_stack_trace.close_last(|#[cfg_attr(not(feature = "logging"), allow(unused))] details| {
                #[cfg(feature = "logging")]
                self.head.with_update_logger(|logger| {
                    logger.log(UpdateLogEntry::Update(details.clone()));
                })
            });
            let recursive_update_stack_trace = &self.head.recursive_update_stack_trace;
            assert!(recursive_update_stack_trace.len() < VMode::max_recursive_updates_before_loop_detected(), "update loop detected:\n{}", recursive_update_stack_trace);

            if self.head.node.is_none() {
                // Do construct
                self.do_update(|self_| {
                    // Actually do construct and set node
                    let mut node = self_.construct.construct(&mut self_.head);

                    // from devolve-ui.js: "Create pixi if pixi component and on web"
                    // should have a hook in view trait we can call through node.view().hook(...)

                    // Update children (if box or another component)
                    node.update(self_);

                    self_.head.node = Some(node)
                })
            } else {
                // Reset
                self.run_update_destructors();
                self.head.h.next_state_index = 0;
                // self.head.h.provided_contexts.clear();

                // Do construct
                self.do_update(|self_| {
                    let mut node = self_.construct.construct(&mut self_.head);

                    // from devolve-ui.js: "Update pixi if pixi component and on web"
                    // should have a hook in view trait we can call through node.view().hook(...)

                    // Update children (if box or another component)
                    node.update(self_);

                    self_.head.invalidate();
                    self_.head.node = Some(node)
                })
            }
        }

        self.head.recursive_update_stack_trace.clear()
    }

    /// Destroy the component: run destructors and invalidate + children
    fn destroy(mut self: Box<Self>) {
        assert!(self.head.node.is_some(), "tried to destroy uninitialized component");

        self.run_update_destructors();
        self.run_permanent_destructors();

        // from devolve-ui.js: "Destroy pixi if pixi component and on web"
        // should have a hook in view trait we can call through node.view().hook(...)

        self.head.invalidate();

        for child in self.head.children.into_values() {
            child.destroy()
        }
    }

    /// `body` will change the component's `node`, and this does other necessary changes.
    fn do_update(mut self: &mut Box<Self>, body: impl FnOnce(&mut Box<Self>) -> ()) {
        self.head.is_being_updated = true;

        body(self);

        self.clear_fresh_and_remove_stale_children();
        self.head.is_being_updated = false;
        self.run_effects();
    }

    /// Remove children who weren't re-used in the update.
    /// For children that were reused, `is_fresh` will be true, and then this sets it to `false`
    /// so in the next update if they aren't re-used they will be removed.
    fn clear_fresh_and_remove_stale_children(self: &mut Box<Self>) {
        let child_keys: Vec<VComponentKey> = self.head.children.keys().cloned().collect();
        for child_key in child_keys {
            let child = self.head.children.get_mut(&child_key).unwrap();
            if child.head.is_fresh {
                child.head.is_fresh = false
            } else {
                let child = self.head.children.remove(&child_key).unwrap();
                child.destroy();
            }
        }
    }

    /// Runs effects: forwards to `construct`.
    fn run_effects(self: &mut Box<Self>) {
        self.construct.run_effects(&mut self.head);
    }

    /// Runs update destructors: forwards to `construct`.
    fn run_update_destructors(self: &mut Box<Self>) {
        self.construct.run_update_destructors(&mut self.head);
    }

    /// Runs permanent destructors: forwards to `construct`.
    fn run_permanent_destructors(self: &mut Box<Self>) {
        self.construct.run_permanent_destructors(&mut self.head);
    }

    /// Child component with the given key
    pub(super) fn child_mut<'a>(self: &'a mut Box<Self>, key: &VComponentKey) -> Option<&'a mut Box<VComponent<ViewData>>> {
        self.head.children.get_mut(key)
    }

    /// Descendent with the given path.
    pub(in crate::core) fn down_path_mut<'a>(self: &'a mut Box<Self>, path: &VComponentPath) -> Option<&'a mut Box<VComponent<ViewData>>> {
        let mut current = self;
        for segment in path.iter() {
            current = current.child_mut(segment)?;
        }
        Some(current)
    }
}

impl <ViewData: VViewData> VComponentHead<ViewData> {
    /// Mark that the component has pending updates if not already marked.
    /// `details` is used for the debug message if we detect an infinite loop.
    pub(in crate::core) fn update(&mut self, details: UpdateDetails) {
        self.has_pending_updates = true;
        if VMode::is_debug() {
            self.recursive_update_stack_trace.add_to_last(details);
        }
    }

    /// Mark that this component needs updates and its view is stale.
    fn invalidate(&self) {
        if let Some(renderer) = self.renderer.upgrade() {
            renderer.invalidate(self.path(), self.view());
        }
    }

    /// A flag for another thread or time which, when set,
    /// marks that this component needs updates and its view is stale (if it still exists).
    pub(in crate::core) fn invalidate_flag(&self) -> NeedsUpdateFlag {
        match self.renderer.upgrade() {
            None => NeedsUpdateFlag::empty(self.path(), self.view().id()),
            Some(renderer) => renderer.invalidate_flag_for(self.path(), self.view())
        }
    }

    /// Id unique for every component and view.
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// Key which identifies this component in updates.
    pub fn key(&self) -> VComponentKey {
        self.key.clone()
    }

    /// Path which identifies this component from the root, by following keys.
    pub(super) fn path(&self) -> VComponentPath {
        self.parent_path.clone() + self.key.clone()
    }

    /// Child component's head
    #[allow(clippy::needless_lifetimes)]
    pub(super) fn child<'a>(&'a self, key: &VComponentKey) -> Option<&'a VComponentHead<ViewData>> {
        match self.children.get(key) {
            None => None,
            Some(component) => Some(&component.head)
        }
    }

    /// Add a new child component.
    pub(super) fn add_child(&mut self, child: Box<VComponent<ViewData>>) -> &Box<VComponent<ViewData>> {
        let key = child.head.key.clone();
        let old_value = self.children.insert(key.clone(), child);
        assert!(old_value.is_none(), "child with key {} added twice", key);
        self.children.get(&key).unwrap()
    }

    /// Reference to this `VComponent` which can be cloned and lifetime extended.
    /// When you want to get the `VComponent` back you can call `with`.
    ///
    /// **Warning:** Calling `with` on multiple components at the same time (e.g. nested) will cause a runtime error.
    pub(super) fn vref(&self) -> VComponentRef<ViewData> {
        VComponentRef {
            renderer: self.renderer.clone(),
            path: self.path()
        }
    }

    /// Is the component being created? Otherwise it's already created but may or may not be updating.
    pub(in crate::core) fn is_being_created(&self) -> bool {
        self.node.is_none()
    }

    /// Gets the components root child view and that view's actual component:
    /// If the `node` is another component, recurses, and otherwise the component will be `self`.
    #[allow(clippy::needless_lifetimes)]
    pub(in crate::core) fn component_and_view<'a>(&'a self) -> VComponentAndView<'a, ViewData> {
        self.node.as_ref().expect("tried to get view of uninitialized component").component_and_view(self)
    }

    /// Gets the components root child view: if the `node` is another component, recurses.
    #[allow(clippy::needless_lifetimes)]
    pub(in crate::core) fn view<'a>(&'a self) -> &'a Box<VView<ViewData>> {
        self.node.as_ref().expect("tried to get view of uninitialized component").view(self)
    }

    /// Gets the renderer, which has the root component and controls rendering.
    pub(in crate::core) fn renderer(&self) -> Weak<dyn VComponentRoot<ViewData = ViewData>> {
        self.renderer.clone()
    }

    #[cfg(feature = "logging")]
    fn with_update_logger(&self, action: impl FnOnce(&mut UpdateLogger<ViewData>)) {
        if VMode::is_logging() {
            if let Some(renderer) = self.renderer.upgrade() {
                renderer.with_update_logger(action)
            }
        }
    }
}

impl <ViewData: VViewData> dyn VComponentConstruct<ViewData=ViewData> {
    /// Runtime-cast `props` to whatever we want, but it panics if they're not the same type.
    #[allow(clippy::needless_lifetimes)]
    pub(super) fn cast_props<'a, Props: Any>(&'a self) -> &'a Props {
        self.props().downcast_ref().expect("props casted to the wrong type")
    }
}

impl <Props: Any, ViewData: VViewData, F: Fn(VComponentContext2<'_, Props, ViewData>) -> VNode<ViewData> + 'static> VComponentConstruct for VComponentConstructImpl<Props, ViewData, F> {
    type ViewData = ViewData;

    fn props(&self) -> &dyn Any {
        &self.props
    }

    fn construct(&mut self, component: &mut VComponentHead<Self::ViewData>) -> VNode<Self::ViewData> {
        let construct = &self.construct;
        construct((VComponentContext1 {
            component,
            effects: &mut self.effects,
            phantom: PhantomData
        }, &self.props))
    }

    fn run_effects(&mut self, component: &mut VComponentHead<Self::ViewData>) {
        while let Some(effect) = self.effects.effects.pop() {
            if component.has_pending_updates {
                break
            }

            effect((VEffectContext1 {
                component,
                destructors: &mut self.destructors,
                phantom: PhantomData
            }, &self.props));
        }
    }

    fn run_update_destructors(&mut self, component: &mut VComponentHead<Self::ViewData>) {
        while let Some(update_destructor) = self.destructors.update_destructors.pop() {
            update_destructor((VDestructorContext1 {
                component,
                phantom: PhantomData
            }, &self.props))
        }
        self.destructors.update_destructors.append(&mut self.destructors.next_update_destructors);
    }

    fn run_permanent_destructors(&mut self, component: &mut VComponentHead<Self::ViewData>) {
        while let Some(permanent_destructor) = self.destructors.permanent_destructors.pop() {
            permanent_destructor((VDestructorContext1 {
                component,
                phantom: PhantomData
            }, &self.props))
        }
    }
}


impl <Props: Any, ViewData: VViewData> VComponentEffects<Props, ViewData> {
    pub fn new() -> Self {
        Self {
            effects: Vec::new()
        }
    }
}

impl <Props: Any, ViewData: VViewData> VComponentDestructors<Props, ViewData> {
    pub fn new() -> Self {
        Self {
            update_destructors: Vec::new(),
            next_update_destructors: Vec::new(),
            permanent_destructors: Vec::new()
        }
    }
}
// endregion

// region boilerplate Debug impls
impl <ViewData: VViewData + Debug> Debug for VComponent<ViewData> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VComponent")
            .field("head", &self.head)
            .field("construct", &self.construct)
            .finish()
    }
}

impl <ViewData: VViewData + Debug> Debug for VComponentHead<ViewData> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VComponentHead")
            .field("id", &self.id)
            .field("key", &self.key)
            .field("parent_path", &self.parent_path)
            .field("is_fresh", &self.is_fresh)
            .field("is_being_created", &self.is_being_created())
            .field("is_being_updated", &self.is_being_updated)
            .field("has_pending_updates", &self.has_pending_updates)
            .field("#h.state", &self.h.state.len())
            .field("h.next_state_index", &self.h.next_state_index)
            .field("recursive_update_stack_trace", &self.recursive_update_stack_trace)
            .field("node", &self.node)
            .finish_non_exhaustive()
    }
}

impl <Props: Any, ViewData: VViewData, F: Fn(VComponentContext2<'_, Props, ViewData>) -> VNode<ViewData> + 'static> Debug for VComponentConstructImpl<Props, ViewData, F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VComponentConstructImpl")
            .field("effects", &self.effects)
            .finish_non_exhaustive()
    }
}

impl <Props: Any, ViewData: VViewData> Debug for VComponentEffects<Props, ViewData> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VComponentEffects")
            .field("#effects", &self.effects.len())
            .finish()
    }
}

impl <Props: Any, ViewData: VViewData> Debug for VComponentDestructors<Props, ViewData> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VComponentDestructors")
            .field("#update_destructors", &self.update_destructors.len())
            .field("#next_update_destructors", &self.next_update_destructors.len())
            .field("#permanent_destructors", &self.permanent_destructors.len())
            .finish()
    }
}
// endregion