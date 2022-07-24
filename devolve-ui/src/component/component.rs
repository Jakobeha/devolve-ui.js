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

use crate::component::parent::VParent;
use crate::component::mode::VMode;
use crate::component::node::{NodeId, VComponentAndView, VNode};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::{Rc, Weak};
use std::fmt::{Debug, Formatter};
use std::iter::once;
use std::mem;
use std::mem::MaybeUninit;
use crate::component::context::{VComponentContext1, VComponentContext2, VDestructorContext1, VDestructorContext2, VEffectContext1, VEffectContext2};
use crate::component::path::{VComponentKey, VComponentPath, VComponentRef, VComponentRefResolved};
use crate::component::root::VComponentRoot;
use crate::component::update_details::{UpdateBacktrace, UpdateDetails, UpdateStack};
use crate::hooks::provider::UntypedProviderId;
#[cfg(feature = "logging")]
use crate::logging::update_logger::UpdateLogger;
use crate::misc::hash_map_ref_stack::HashMapWithAssocMutStack;
use crate::renderer::stale_data::NeedsUpdateFlag;
use crate::view::view::{VView, VViewData};

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

    local_update_stack: Option<UpdateStack>,
    is_fresh: bool,
}

pub(in crate::core) struct VComponentStateData {
    pub state: Vec<Box<dyn Any>>,
    pub next_state_index: usize,
    // pub(in crate::core) provided_contexts: HashMap<Context, Box<dyn Any>>,
    // pub(in crate::core) consumed_contexts: HashMap<Context, Box<dyn Any>>
}

#[derive(Debug)]
pub struct ContextPendingUpdates {
    pub update_details: Option<Vec<UpdateDetails>>,
    // These next 2 properties are redundant but to VComponentHead but because we don't pass it
    // in associated data, we need them. Otherwise we need some annoying code restructuring
    pub path: VComponentPath
}

pub type VComponentLocalContexts = HashMap<UntypedProviderId, Box<dyn Any>>;
pub type VComponentContexts<'a> = HashMapWithAssocMutStack<'a, UntypedProviderId, Box<dyn Any>, ContextPendingUpdates>;

/// Part of the component with data whose size depends on `Props`, so it's a runtime-sized trait object.
pub(super) trait VComponentConstruct: Debug {
    type ViewData: VViewData;

    fn reuse(self: Box<Self>, reconstruct: Box<dyn VComponentReconstruct<ViewData=Self::ViewData>>) -> Box<dyn VComponentConstruct<ViewData=Self::ViewData>>;

    fn props(&self) -> &dyn Any;
    fn construct(&mut self, component: &mut VComponentHead<Self::ViewData>, contexts: &mut VComponentContexts<'_>) -> VNode<Self::ViewData>;

    fn local_contexts(&mut self) -> (&mut VComponentLocalContexts, &mut ContextPendingUpdates);
    fn local_context_changes(&mut self) -> &mut ContextPendingUpdates;
    fn local_contexts_and_props(&mut self) -> (&mut VComponentLocalContexts, &mut ContextPendingUpdates, &dyn Any);

    fn run_effects(&mut self, component: &mut VComponentHead<Self::ViewData>, contexts: &mut VComponentContexts<'_>);
    fn run_update_destructors(&mut self, component: &mut VComponentHead<Self::ViewData>, contexts: &mut VComponentContexts<'_>);
    fn run_permanent_destructors(&mut self, component: &mut VComponentHead<Self::ViewData>, contexts: &mut VComponentContexts<'_>);
}

pub(super) trait VComponentReconstruct {
    type ViewData: VViewData;

    unsafe fn _complete(self: Box<Self>, props_id: TypeId, s: *mut ()) -> Box<dyn VComponentConstruct<ViewData=Self::ViewData>>;
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
                            found_child.update(contexts, once(UpdateDetails::Reuse {
                                key,
                                backtrace: UpdateBacktrace::here()
                            }));
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
                let mut component = Self::create(parent, key, props, construct);
                // Create = needs update
                component.update(contexts, once(UpdateDetails::CreateNew {
                    key,
                    backtrace: UpdateBacktrace::here()
                }));
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

                is_fresh: true,
                local_update_stack: None
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
    pub(in crate::core) fn update(mut self: &mut Box<Self>, contexts: &mut VComponentContexts<'_>, initial_updates: impl Iterator<Item=UpdateDetails>) {
        assert!(!self.head.is_being_updated(), "why is local_update_stack not empty when calling update? Are we calling it nested?");
        assert!(!self.construct.local_context_changes().is_being_updated(), "why is construct's local_update_stack not empty when calling update? Are we calling it nested?");

        let old_view_id = self.head.try_view().map_or(NodeId::NULL, |view| view.id());

        let mut update_stack = UpdateStack::new();
        update_stack.add_all_to_last(initial_updates);
        self.head.local_update_stack = Some(update_stack);
        self.construct.local_context_changes().update_details = Some(Vec::new());

        loop {
            // Update stack trace stuff

            let local_update_stack = self.head.local_update_stack.as_mut().unwrap();
            local_update_stack.append_to_last(self.construct.local_context_changes().update_details.as_mut().unwrap());
            if !local_update_stack.has_pending() {
                break
            }

            local_update_stack.close_last();
            if local_update_stack.len() == VMode::max_recursive_updates_before_loop_detected() {
                log::error!("update loop detected:\n{}", local_update_stack);
                panic!("update loop detected");
            }

            // Actual update stuff

            // Reset if reconstruct
            if self.head.node.is_some() {
                self.run_update_destructors(contexts);
                self.head.h.next_state_index = 0;
            }

            // Construct or reconstruct
            let node = self.construct.construct(&mut self.head, contexts);
            self.head.node = Some(node);

            // After construct
            self.clear_fresh_and_remove_stale_children(contexts);
            self.run_effects(contexts);
        }

        assert!(!self.head.local_update_stack.as_ref().unwrap().has_pending());
        assert!(self.construct.local_context_changes().update_details.as_ref().unwrap().is_empty());
        self.construct.local_context_changes().update_details = None;
        let local_update_stack = self.head.local_update_stack.take().unwrap();

        if local_update_stack.is_empty() {
            log::warn!("component at path {} updated but it had no pending updates", self.head.path());
        } else {
            self.head.invalidate_view(old_view_id);
        }

        #[cfg(feature = "logging")]
        {
            let path = self.head.path().clone();
            VComponentHead::with_update_logger(&self.head.renderer, move |logger| {
                logger.log_update(path, local_update_stack);
            });
        }
    }

    /// Destroy the component: run destructors and invalidate + children
    fn destroy(mut self: Box<Self>, contexts: &mut VComponentContexts<'_>) {
        assert!(self.head.node.is_some(), "tried to destroy uninitialized component");

        self.run_update_destructors(contexts);
        self.run_permanent_destructors(contexts);

        // from devolve-ui.js: "Destroy pixi if pixi component and on web"
        // should have a hook in view trait we can call through node.view().hook(...)

        // We need to explicitly invalidate because it's not an update
        self.head.invalidate_view(self.head.view().id());

        for child in self.head.children.into_values() {
            let (local_contexts, local_context_changes) = self.construct.local_contexts();
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
    fn local_contexts_and_child_mut<'a>(self: &'a mut Box<Self>, key: &VComponentKey) -> (&'a mut VComponentLocalContexts, &'a mut ContextPendingUpdates, Option<&'a mut Box<VComponent<ViewData>>>) {
        let (local_contexts, local_context_changes) = self.construct.local_contexts();
        (local_contexts, local_context_changes, self.head.children.get_mut(key))
    }

    /// Descendent with the given path.
    pub(in crate::core) fn down_path_mut<'a>(self: &'a mut Box<Self>, path: &VComponentPath, mut is_first: bool, mut parents: Vec<(&'a mut VComponentLocalContexts, &'a mut ContextPendingUpdates)>) -> Option<VComponentRefResolved<'a, ViewData>> {
        let mut current = self;
        for segment in path.iter() {
            if is_first {
                assert_eq!(segment, current.head.key(), "component is not head of path");
                is_first = false;
                continue;
            }

            let (local_contexts, changes, child) = current.local_contexts_and_child_mut(segment);
            parents.push((local_contexts, changes));
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
        if let Some(local_update_stack) = &mut self.local_update_stack { // is_being_updated
            local_update_stack.add_to_last(details);
        } else {
            if let Some(renderer) = self.renderer.upgrade() {
                renderer.queue_needs_update(self.path(), details);
            } else {
                log::warn!("component at path {} got pending update {} but no renderer", self.path(), details);
            }
        }
    }

    /// Mark that the view with given id (the component's old view) is stale and should be uncached.
    fn invalidate_view(&self, view_id: NodeId) {
        if let Some(renderer) = self.renderer.upgrade() {
            renderer.invalidate_view(view_id);
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

    /// Add a new child component.
    pub(super) fn add_child(&mut self, child: Box<VComponent<ViewData>>) -> &Box<VComponent<ViewData>> {
        let key = *child.head.key();
        let old_value = self.children.insert(key.clone(), child);
        assert!(old_value.is_none(), "child with key {} added twice", key);
        self.children.get(&key).unwrap()
    }

    /// Reference to this `VComponent` which can be cloned and lifetime extended.
    /// When you want to get the `VComponent` back you can call `with`.
    ///
    /// **Warning:** Calling `with` on multiple components at the same time (e.g. nested) will cause a runtime error.
    pub fn vref(&self) -> VComponentRef<ViewData> {
        VComponentRef {
            renderer: self.renderer.clone(),
            path: self.path().clone()
        }
    }

    /// Is the component being created? Otherwise it's already created but may or may not be updating.
    pub(in crate::core) fn is_being_created(&self) -> bool {
        self.node.is_none()
    }

    /// Is the component being updated? If so pending updates will be added to the local update queue,
    /// otherwise they go to the root update queue.
    pub(in crate::core) fn is_being_updated(&self) -> bool {
        self.local_update_stack.is_some()
    }

    /// Gets the components root child view and that view's actual component:
    /// If the `node` is another component, recurses, and otherwise the component will be `self`.
    #[allow(clippy::needless_lifetimes)]
    pub(in crate::core) fn component_and_view<'a>(&'a self) -> VComponentAndView<'a, ViewData> {
        self.node.as_ref().expect("tried to get view of uninitialized component").component_and_view(self)
    }

    /// Gets the components root child view: if the `node` is another component, recurses.
    /// *Panics* if the component is not initialized
    #[allow(clippy::needless_lifetimes)]
    pub(in crate::core) fn view<'a>(&'a self) -> &'a Box<VView<ViewData>> {
        self.node.as_ref().expect("tried to get view of uninitialized component").view(self)
    }

    /// Gets the components root child view: if the `node` is another component, recurses.
    #[allow(clippy::needless_lifetimes)]
    pub(super) fn try_view<'a>(&'a self) -> Option<&'a Box<VView<ViewData>>> {
        self.node.as_ref().and_then(|node| node.try_view(self))
    }

    /// Gets the renderer, which has the root component and controls rendering.
    pub(in crate::core) fn renderer(&self) -> Weak<dyn VComponentRoot<ViewData = ViewData>> {
        self.renderer.clone()
    }

    #[cfg(feature = "logging")]
    // Would use self and self.renderer instead, but we need to simultanoeusly borrow local_update_stack
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
    pub(super) fn local_contexts_and_cast_props<'a, Props: Any>(&'a mut self) -> (&'a mut VComponentLocalContexts, &'a mut ContextPendingUpdates, &'a Props) {
        let (local_contexts, local_context_changes, props) = self.local_contexts_and_props();
        let props = props.downcast_ref().expect("props casted to the wrong type");
        (local_contexts, local_context_changes, props)
    }
}

impl <Props: Any, ViewData: VViewData, F: Fn(VComponentContext2<'_, '_, Props, ViewData>) -> VNode<ViewData> + 'static> VComponentConstruct for VComponentConstructImpl<Props, ViewData, F> {
    type ViewData = ViewData;

    fn reuse(self: Box<Self>, reconstruct: Box<dyn VComponentReconstruct<ViewData=ViewData>>) -> Box<dyn VComponentConstruct<ViewData=ViewData>> {
        reconstruct.complete::<Props>(self.s)
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

    fn local_contexts(&mut self) -> (&mut VComponentLocalContexts, &mut ContextPendingUpdates) {
        (&mut self.s.local_contexts, &mut self.s.pending_updates)
    }

    fn local_context_changes(&mut self) -> &mut ContextPendingUpdates {
        &mut self.s.pending_updates
    }

    fn local_contexts_and_props(&mut self) -> (&mut VComponentLocalContexts, &mut ContextPendingUpdates, &dyn Any) {
        (&mut self.s.local_contexts, &mut self.s.pending_updates, &self.props)
    }

    fn run_effects(&mut self, component: &mut VComponentHead<Self::ViewData>, contexts: &mut VComponentContexts<'_>) {
        contexts.with_push(&mut self.s.local_contexts, &mut self.s.pending_updates, |contexts| {
            while let Some(effect) = self.s.effects.effects.pop() {
                let update_details = &contexts.top_mut().unwrap().1.update_details;
                // If we have pending updates, we break, because immediately after this function ends
                // we run the pending updates and then call run_effects again to run remanining effects.
                // This is so effects don't have stale data (?).
                if component.local_update_stack.is_some_and(|local_update_stack| local_update_stack.has_pending()) ||
                    update_details.is_some_and(|update_details| !update_details.is_empty()) {
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

impl <ViewData: VViewData> dyn VComponentReconstruct<ViewData=ViewData> {
    fn complete<Props: Any>(self: Box<Self>, s: VComponentConstructState<Props, ViewData>) -> Box<dyn VComponentConstruct<ViewData=ViewData>> {
        let mut s = MaybeUninit::new(s);
        let s = &mut s as *mut _ as *mut ();
        unsafe { self._complete(TypeId::of::<Props>(), s) }
    }
}

impl <Props: Any, ViewData: VViewData + 'static, F: Fn(VComponentContext2<'_, '_, Props, ViewData>) -> VNode<ViewData> + 'static> VComponentReconstruct for VComponentReconstructImpl<Props, ViewData, F> {
    type ViewData = ViewData;

    unsafe fn _complete(self: Box<Self>, props_id: TypeId, s: *mut ()) -> Box<dyn VComponentConstruct<ViewData=ViewData>> {
        assert_eq!(props_id, TypeId::of::<Props>(), "props type mismatch (expected != actual)");
        let s: VComponentConstructState<Props, ViewData> = mem::replace(
            &mut *(s as *mut MaybeUninit<VComponentConstructState<Props, ViewData>>),
            MaybeUninit::uninit()
        ).assume_init();
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
            update_details: None,
            path,
        }
    }

    pub(in crate::core) fn pending_update<ViewData: VViewData>(&mut self, details: UpdateDetails, renderer: Weak<dyn VComponentRoot<ViewData=ViewData>>) {
        if let Some(update_details) = &mut self.update_details { // is_being_updated
            update_details.push(details);
        } else {
            // This happens when we've got a context update from an async update.
            // Otherwise the parent is being updated, and we've got a context update from a child update from a parent update.
            if let Some(renderer) = renderer.upgrade() {
                renderer.queue_needs_update(&self.path, details)
            } else {
                log::warn!("component (from context so path unknown) got update with no renderer: {}", details);
            }
        }
    }

    pub(in crate::core) fn is_being_updated(&self) -> bool {
        self.update_details.is_some()
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
            .field("path", &self.path)
            .field("is_fresh", &self.is_fresh)
            .field("local_update_stack", &self.local_update_stack)
            .field("node", &self.node)
            .field("#h.state", &self.h.state.len())
            .field("h.next_state_index", &self.h.next_state_index)
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