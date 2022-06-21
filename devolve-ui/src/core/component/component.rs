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
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::{Rc, Weak};
use std::fmt::{Debug, Formatter};
use std::mem;
use crate::core::component::context::{VComponentContext1, VComponentContext2, VDestructorContext1, VDestructorContext2, VEffectContext1, VEffectContext2};
use crate::core::component::path::{VComponentKey, VComponentPath, VComponentRef, VComponentRefResolved};
use crate::core::component::root::VComponentRoot;
use crate::core::component::update_details::{UpdateDetails, UpdateFrame, UpdateStack};
use crate::core::hooks::context::AnonContextId;
#[cfg(feature = "logging")]
use crate::core::logging::update_logger::{UpdateLogEntry, UpdateLogger};
use crate::core::misc::hash_map_ref_stack::{HashMapMutStack, HashMapWithAssocMutStack};
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
    /* readonly pub */  path: VComponentPath,

    node: Option<VNode<ViewData>>,

    /*readonly*/ children: HashMap<VComponentKey, Box<VComponent<ViewData>>>,
    /*readonly*/ renderer: Weak<dyn VComponentRoot<ViewData = ViewData>>,

    pub(in crate::core) h: VComponentStateData,

    is_being_updated: bool,
    is_fresh: bool,
    recursive_update_stack_trace: UpdateStack,
}

pub(in crate::core) struct VComponentStateData {
    pub state: Vec<Box<dyn Any>>,
    pub next_state_index: usize,
    // pub(in crate::core) provided_contexts: HashMap<Context, Box<dyn Any>>,
    // pub(in crate::core) consumed_contexts: HashMap<Context, Box<dyn Any>>
}

#[derive(Debug)]
pub struct ContextPendingUpdates {
    pub update_details: Vec<UpdateDetails>,
    // These next 2 properties are redundant but to VComponentHead but because we don't pass it
    // in associated data, we need them. Otherwise we need some annoying code restructuring
    pub path: VComponentPath,
    pub is_being_updated: bool
}

pub type VComponentLocalContexts = HashMap<AnonContextId, Box<dyn Any>>;
pub type VComponentContexts<'a> = HashMapWithAssocMutStack<'a, AnonContextId, Box<dyn Any>, ContextPendingUpdates>;

/// Part of the component with data whose size depends on `Props`, so it's a runtime-sized trait object.
pub(super) trait VComponentConstruct: Debug {
    type ViewData: VViewData;

    fn reuse(self: Box<Self>, reconstruct: Box<dyn VComponentReconstruct<ViewData=Self::ViewData>>) -> Box<dyn VComponentConstruct<ViewData=Self::ViewData>>;

    fn props(&self) -> &dyn Any;
    fn construct(&mut self, component: &mut VComponentHead<Self::ViewData>, contexts: &mut VComponentContexts<'_>) -> VNode<Self::ViewData>;

    fn local_contexts_and_changes(&mut self) -> (&mut VComponentLocalContexts, &mut ContextPendingUpdates);
    fn local_context_changes(&mut self) -> &mut ContextPendingUpdates;
    fn local_contexts_and_props(&mut self) -> (&mut VComponentLocalContexts, &dyn Any);

    fn run_effects(&mut self, component: &mut VComponentHead<Self::ViewData>, contexts: &mut VComponentContexts<'_>);
    fn run_update_destructors(&mut self, component: &mut VComponentHead<Self::ViewData>, contexts: &mut VComponentContexts<'_>);
    fn run_permanent_destructors(&mut self, component: &mut VComponentHead<Self::ViewData>, contexts: &mut VComponentContexts<'_>);
}

pub trait VComponentReconstruct {
    type ViewData: VViewData;

    fn check_downcast(&self, id: TypeId) -> bool;
}

trait VComponentReconstruct2: VComponentReconstruct {
    type Props: Any;

    fn complete(
        self: Box<Self>,
        s: VComponentConstructState<Self::Props, Self::ViewData>
    ) -> Box<dyn VComponentConstruct<ViewData=Self::ViewData>>;
}

/// Part of the component with data whose size depends on `Props`. This is the compile-time sized implementation
/// which requires a specific `Props`.
struct VComponentConstructImpl<Props: Any, ViewData: VViewData, F: Fn(VComponentContext2<'_, '_, Props, ViewData>) -> VNode<ViewData> + 'static> {
    props: Props,
    construct: F,
    s: VComponentConstructState<Props, ViewData>
}

struct VComponentConstructState<Props: Any, ViewData: VViewData> {
    pub local_contexts: VComponentLocalContexts,
    pub pending_updates: ContextPendingUpdates,
    pub effects: VComponentEffects<Props, ViewData>,
    pub destructors: VComponentDestructors<Props, ViewData>
}

struct VComponentReconstructImpl<Props: Any, ViewData: VViewData, F: Fn(VComponentContext2<'_, '_, Props, ViewData>) -> VNode<ViewData> + 'static> {
    props: Props,
    construct: F,
    phantom: PhantomData<ViewData>
}

pub(in crate::core) struct VComponentEffects<Props: Any, ViewData: VViewData> {
    pub effects: Vec<Box<dyn Fn(VEffectContext2<'_, '_, Props, ViewData>) -> ()>>,
}

pub(in crate::core) struct VComponentDestructors<Props: Any, ViewData: VViewData> {
    pub update_destructors: Vec<Box<dyn FnOnce(VDestructorContext2<'_, '_, Props, ViewData>) -> ()>>,
    pub next_update_destructors: Vec<Box<dyn FnOnce(VDestructorContext2<'_, '_, Props, ViewData>) -> ()>>,
    pub permanent_destructors: Vec<Box<dyn FnOnce(VDestructorContext2<'_, '_, Props, ViewData>) -> ()>>
}
// endregion

// region impls
impl <ViewData: VViewData + 'static> VComponent<ViewData> {
    /// Create a new component *or* update and reuse the existing component, if it has the same parent or key.
    pub(in crate::core) fn new<Props: 'static, F: Fn(VComponentContext2<'_, '_, Props, ViewData>) -> VNode<ViewData> + 'static>(
        parent: VParent<'_, ViewData>,
        contexts: &mut VComponentContexts<'_>,
        key: VComponentKey,
        props: Props,
        construct: F
    ) -> Box<VComponent<ViewData>> {
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
                            found_child.construct = found_child.construct.reuse(Box::new(VComponentReconstructImpl {
                                props,
                                construct,
                                phantom: PhantomData
                            }));
                            found_child.head.is_fresh = true;
                            found_child.head.pending_update(UpdateDetails::Reuse);
                            found_child.update(contexts);
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
            Action::Create(parent, props, construct) => {
                let component = Self::create(parent, key, props, construct);
                component.head.pending_update(UpdateDetails::CreateNew);
                component.update(contexts);
                component
            }
        }
    }

    /// Create a new component with the parent and key. Don't call if there is already an existing component.
    fn create<Props: 'static, F: Fn(VComponentContext2<'_, '_, Props, ViewData>) -> VNode<ViewData> + 'static>(parent: VParent<'_, ViewData>, key: VComponentKey, props: Props, construct: F) -> Box<Self>{
        let path = parent.path() + key;
        Box::new(VComponent {
            head: VComponentHead {
                id: NodeId::next(),
                path: path.clone(),
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
                recursive_update_stack_trace: {
                    let mut stack = UpdateStack::new();
                    stack.add_to_last(UpdateDetails::Create(key));
                    stack
                }
            },
            construct: Box::new(VComponentConstructImpl {
                props,
                construct,
                s: VComponentConstructState::new(path)
            })
        })
    }
}

impl <ViewData: VViewData> VComponent<ViewData> {
    /// Run pending updates on this component.
    /// If there are any pending updates, updates will also be run on children.
    /// Otherwise they won't but also this should never be called if there are no pending updates.
    pub(in crate::core) fn update(mut self: &mut Box<Self>, contexts: &mut VComponentContexts<'_>) {
        self.head.is_being_updated = true;
        self.construct.local_context_changes().is_being_updated = true;
        loop {
            // Update stack trace stuff

            self.head.recursive_update_stack_trace.append_to_last(&mut self.construct.local_context_changes().update_details);
            if !self.head.recursive_update_stack_trace.has_pending() {
                break
            }

            self.head.recursive_update_stack_trace.close_last();
            let recursive_update_stack_trace = &self.head.recursive_update_stack_trace;
            assert!(recursive_update_stack_trace.len() < VMode::max_recursive_updates_before_loop_detected(), "update loop detected:\n{}", recursive_update_stack_trace);

            // Actual update stuff

            // Reset if reconstruct
            if self.head.node.is_some() {
                self.run_update_destructors(contexts);
                self.head.h.next_state_index = 0;
            }

            // Construct or reconstruct
            let mut node = self.construct.construct(&mut self.head, contexts);
            self.head.node = Some(node);

            // After construct
            self.clear_fresh_and_remove_stale_children(contexts);
            self.run_effects(contexts);
        }
        self.construct.local_context_changes().is_being_updated = false;
        self.head.is_being_updated = false;

        VComponentHead::with_update_logger(&self.head.renderer, |logger| {
            logger.log_update(self.head.path().clone(), self.head.recursive_update_stack_trace.clone());
        });

        if self.head.recursive_update_stack_trace.is_empty() {
            eprintln!("WARNING: component at path {} updated but it had no pending updates", self.head.path());
        } else {
            self.head.recursive_update_stack_trace.clear();
            self.head.invalidate_view();
        }
    }

    /// Destroy the component: run destructors and invalidate + children
    fn destroy(mut self: Box<Self>, contexts: &mut VComponentContexts<'_>) {
        assert!(self.head.node.is_some(), "tried to destroy uninitialized component");

        self.run_update_destructors(contexts);
        self.run_permanent_destructors(contexts);

        // from devolve-ui.js: "Destroy pixi if pixi component and on web"
        // should have a hook in view trait we can call through node.view().hook(...)

        // We need to explicitly call because this is not an update
        self.head.invalidate_view();

        for child in self.head.children.into_values() {
            let (local_contexts, local_context_changes) = self.construct.local_contexts_and_changes();
            contexts.with_push(local_contexts, local_context_changes, |contexts| {
                child.destroy(contexts)
            });
        }
    }

    /// Remove children who weren't re-used in the update.
    /// For children that were reused, `is_fresh` will be true, and then this sets it to `false`
    /// so in the next update if they aren't re-used they will be removed.
    fn clear_fresh_and_remove_stale_children(self: &mut Box<Self>, contexts: &mut VComponentContexts<'_>) {
        let child_keys: Vec<VComponentKey> = self.head.children.keys().cloned().collect();
        for child_key in child_keys {
            let child = self.head.children.get_mut(&child_key).unwrap();
            if child.head.is_fresh {
                child.head.is_fresh = false
            } else {
                let child = self.head.children.remove(&child_key).unwrap();
                child.destroy(contexts);
            }
        }
    }

    /// Runs effects: forwards to `construct`.
    fn run_effects(self: &mut Box<Self>, contexts: &mut VComponentContexts<'_>) {
        self.construct.run_effects(&mut self.head, contexts);
    }

    /// Runs update destructors: forwards to `construct`.
    fn run_update_destructors(self: &mut Box<Self>, contexts: &mut VComponentContexts<'_>) {
        self.construct.run_update_destructors(&mut self.head, contexts);
    }

    /// Runs permanent destructors: forwards to `construct`.
    /// Runs permanent destructors: forwards to `construct`.
    fn run_permanent_destructors(self: &mut Box<Self>, contexts: &mut VComponentContexts<'_>) {
        self.construct.run_permanent_destructors(&mut self.head, contexts);
    }

    /// Child component with the given key
    fn local_contexts_and_child_mut<'a>(self: &'a mut Box<Self>, key: &VComponentKey) -> (&'a mut VComponentLocalContexts, Option<&'a mut Box<VComponent<ViewData>>>) {
        (self.construct.local_contexts(), self.head.children.get_mut(key))
    }

    /// Descendent with the given path.
    pub(in crate::core) fn down_path_mut<'a>(self: &'a mut Box<Self>, path: &VComponentPath, mut parents: Vec<&'a mut VComponentLocalContexts>) -> Option<VComponentRefResolved<'a, ViewData>> {
        let mut current = self;
        for segment in path.iter() {
            let (local_contexts, child) = current.local_contexts_and_child_mut(segment);
            parents.push(local_contexts);
            current = child?;
        }
        let result = VComponentRefResolved {
            parent_contexts: parents,
            component: current
        };
        Some(result)
    }
}

impl <ViewData: VViewData> VComponentHead<ViewData> {
    /// Mark that the component has a pending update.
    /// `details` is used for the debug message if we detect an infinite loop.
    pub(in crate::core) fn pending_update(&mut self, details: UpdateDetails) {
        self.recursive_update_stack_trace.add_to_last(details);
        if let Some(renderer) = self.renderer.upgrade() {
            renderer.queue_needs_update(self.path());
        }
    }

    /// Mark that this component's view is stale.
    fn invalidate_view(&self) {
        if let Some(renderer) = self.renderer.upgrade() {
            renderer.invalidate_view(self.view());
        }
    }

    /// A flag for another thread or time which, when set,
    /// marks that this component needs updates and its view is stale (if it still exists).
    pub(in crate::core) fn needs_update_flag(&self) -> NeedsUpdateFlag {
        match self.renderer.upgrade() {
            None => NeedsUpdateFlag::empty(self.path().clone()),
            Some(renderer) => renderer.needs_update_flag_for(self.path().clone())
        }
    }

    /// Id unique for every component and view.
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// Key which identifies this component in updates.
    pub fn key(&self) -> &VComponentKey {
        self.path.last().unwrap()
    }

    /// Path which identifies this component from the root, by following keys
    pub fn path(&self) -> &VComponentPath {
        &self.path
    }

    /// Child component's head
    #[allow(clippy::needless_lifetimes)]
    pub(super) fn child_head<'a>(&'a self, key: &VComponentKey) -> Option<&'a VComponentHead<ViewData>> {
        match self.children.get(key) {
            None => None,
            Some(component) => Some(&component.head)
        }
    }

    /// Child component with the given key
    #[allow(clippy::needless_lifetimes)]
    pub(super) fn child_mut<'a>(&'a mut self, key: &VComponentKey) -> Option<&'a mut Box<VComponent<ViewData>>> {
        self.children.get_mut(key)
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
            path: self.path().clone()
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
    // Would use self and self.renderer instead, but we need to simultanoeusly borrow recursive_update_stack_trace
    fn with_update_logger(renderer: &Weak<dyn VComponentRoot<ViewData=ViewData>>, action: impl FnOnce(&mut UpdateLogger<ViewData>)) {
        if VMode::is_logging() {
            if let Some(renderer) = renderer.upgrade() {
                renderer.with_update_logger(action)
            }
        }
    }
}

impl <ViewData: VViewData> dyn VComponentConstruct<ViewData=ViewData> {
    /// Runtime-cast `props` to whatever we want, but it panics if they're not the same type.
    #[allow(clippy::needless_lifetimes)]
    pub(super) fn local_contexts_and_cast_props<'a, Props: Any>(&'a mut self) -> (&'a mut VComponentLocalContexts, &'a Props) {
        let (local_contexts, props) = self.local_contexts_and_props();
        let props = props.downcast_ref().expect("props casted to the wrong type");
        (local_contexts, props)
    }
}

impl <ViewData: VViewData> dyn VComponentReconstruct<ViewData=ViewData> {
    fn force_downcast<Props: Any>(self: Box<Self>) -> Box<dyn VComponentReconstruct2<Props=Props, ViewData=ViewData>> {
        assert!(self.check_downcast(TypeId::of::<Props>()), "component reused with different type of props");
        unsafe { mem::transmute(self) }
    }
}

impl <Props: Any, ViewData: VViewData, F: Fn(VComponentContext2<'_, '_, Props, ViewData>) -> VNode<ViewData> + 'static> VComponentConstruct for VComponentConstructImpl<Props, ViewData, F> {
    type ViewData = ViewData;

    fn reuse(self: Box<Self>, reconstruct: Box<dyn VComponentReconstruct<ViewData=ViewData>>) -> Box<dyn VComponentConstruct<ViewData=ViewData>> {
        let reconstruct = reconstruct.force_downcast::<Props>();
        reconstruct.complete(self.s)
    }

    fn props(&self) -> &dyn Any {
        &self.props
    }

    fn construct(&mut self, component: &mut VComponentHead<Self::ViewData>, contexts: &mut VComponentContexts<'_>) -> VNode<Self::ViewData> {
        let construct = &self.construct;
        contexts.with_push(&mut self.s.local_contexts, &mut self.s.pending_updates, |contexts| {
            construct((VComponentContext1 {
                component,
                contexts,
                effects: &mut self.s.effects,
                phantom: PhantomData
            }, &self.props))
        })
    }

    fn local_contexts_and_changes(&mut self) -> (&mut VComponentLocalContexts, &mut ContextPendingUpdates) {
        (&mut self.s.local_contexts, &mut self.s.pending_updates)
    }

    fn local_context_changes(&mut self) -> &mut ContextPendingUpdates {
        &mut self.s.pending_updates
    }

    fn local_contexts_and_props(&mut self) -> (&mut VComponentLocalContexts, &dyn Any) {
        (&mut self.s.local_contexts, &self.props)
    }

    fn run_effects(&mut self, component: &mut VComponentHead<Self::ViewData>, contexts: &mut VComponentContexts<'_>) {
        contexts.with_push(&mut self.s.local_contexts, &mut self.s.pending_updates, |contexts| {
            while let Some(effect) = self.s.effects.effects.pop() {
                // If we have pending updates, we break, because immediately after this function ends
                // we run the pending updates and then call run_effects again to run remanining effects.
                // This is so effects don't have stale data.
                if component.recursive_update_stack_trace.has_pending() || !self.s.pending_updates.update_details.is_empty() {
                    break
                }

                effect((VEffectContext1 {
                    component,
                    contexts,
                    destructors: &mut self.s.destructors,
                    phantom: PhantomData
                }, &self.props));
            }
        })
    }

    fn run_update_destructors(&mut self, component: &mut VComponentHead<Self::ViewData>, contexts: &mut VComponentContexts) {
        contexts.with_push(&mut self.s.local_contexts, &mut self.s.pending_updates, |contexts| {
            while let Some(update_destructor) = self.s.destructors.update_destructors.pop() {
                update_destructor((VDestructorContext1 {
                    component,
                    contexts,
                    phantom: PhantomData
                }, &self.props));
            }
            self.s.destructors.update_destructors.append(&mut self.s.destructors.next_update_destructors);
        });
    }

    fn run_permanent_destructors(&mut self, component: &mut VComponentHead<Self::ViewData>, contexts: &mut VComponentContexts) {
        contexts.with_push(&mut self.s.local_contexts, &mut self.s.pending_updates, |contexts| {
            while let Some(permanent_destructor) = self.s.destructors.permanent_destructors.pop() {
                permanent_destructor((VDestructorContext1 {
                    component,
                    contexts,
                    phantom: PhantomData
                }, &self.props));
            }
        });
    }
}

impl <Props: Any, ViewData: VViewData, F: Fn(VComponentContext2<'_, '_, Props, ViewData>) -> VNode<ViewData> + 'static> VComponentReconstruct for VComponentReconstructImpl<Props, ViewData, F> {
    type ViewData = ViewData;

    fn check_downcast(&self, id: TypeId) -> bool {
        id == TypeId::of::<Props>()
    }
}

impl <Props: Any, ViewData: VViewData + 'static, F: Fn(VComponentContext2<'_, '_, Props, ViewData>) -> VNode<ViewData> + 'static> VComponentReconstruct2 for VComponentReconstructImpl<Props, ViewData, F> {
    type Props = Props;

    fn complete(
        self: Box<Self>,
        s: VComponentConstructState<Props, ViewData>
    ) -> Box<dyn VComponentConstruct<ViewData=ViewData>> {
        Box::new(VComponentConstructImpl {
            props: self.props,
            construct: self.construct,
            s
        })
    }
}

impl <Props: Any, ViewData: VViewData> VComponentConstructState<Props, ViewData> {
    pub fn new(path: VComponentPath) -> Self {
        VComponentConstructState {
            local_contexts: HashMap::new(),
            pending_updates: ContextPendingUpdates::new(path),
            effects: VComponentEffects::new(),
            destructors: VComponentDestructors::new()
        }
    }
}

impl ContextPendingUpdates {
    fn new(path: VComponentPath) -> Self {
        ContextPendingUpdates {
            update_details: Vec::new(),
            path,
            is_being_updated: false
        }
    }

    pub(in crate::core) fn pending_update<ViewData: VViewData>(&mut self, update_details: UpdateDetails, renderer: Weak<dyn VComponentRoot<ViewData=ViewData>>) {
        self.update_details.push(update_details);
        if !self.is_being_updated {
            // This happens when we've got a context update from an async update.
            // Otherwise the parent is being updated, and we've got a context update from a child update from a parent update.
            if let Some(renderer) = renderer.upgrade() {
                renderer.mark_needs_update(&self.path)
            }
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
            .field("path", &self.path)
            .field("is_fresh", &self.is_fresh)
            .field("is_being_created", &self.is_being_created())
            .field("is_being_updated", &self.is_being_updated)
            .field("recursive_update_stack_trace", &self.recursive_update_stack_trace)
            .field("#h.state", &self.h.state.len())
            .field("h.next_state_index", &self.h.next_state_index)
            .field("node", &self.node)
            .finish_non_exhaustive()
    }
}

impl <Props: Any, ViewData: VViewData, F: Fn(VComponentContext2<'_, '_, Props, ViewData>) -> VNode<ViewData> + 'static> Debug for VComponentConstructImpl<Props, ViewData, F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VComponentConstructImpl")
            .field("s", &self.s)
            .finish_non_exhaustive()
    }
}

impl <Props: Any, ViewData: VViewData> Debug for VComponentConstructState<Props, ViewData> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VComponentConstructImpl")
            .field("#contexts", &self.local_contexts.len())
            .field("effects", &self.effects)
            .field("destructors", &self.destructors)
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