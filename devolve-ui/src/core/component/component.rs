//! Components are the UI elements in devolve-ui.
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
//!
//!

use crate::core::component::parent::VParent;
use crate::core::component::mode::VMode;
use crate::core::component::node::{NodeId, VNode, VComponentAndView};
use std::any::Any;
use std::borrow::Cow;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::rc::{Rc, Weak};
use std::sync::{Weak as WeakArc};
use std::fmt::{Debug, Display, Formatter};
use crate::core::component::context::{VComponentContext1, VComponentContext2, VDestructorContext1, VDestructorContext2, VEffectContext1, VEffectContext2};
use crate::core::component::path::{VComponentKey, VComponentPath};
use crate::core::component::root::VComponentRoot;
use crate::core::misc::notify_flag::NotifyFlag;
use crate::core::view::view::{VView, VViewData};

/// Wrapper for `VNode` so that it's more type-safe,
/// and you don't accidentally use VComponent functions when you meant to use a component itself
#[derive(Debug)]
pub struct VComponentBody<ViewData: VViewData>(VNode<ViewData>);

impl <ViewData: VViewData> VComponentBody<ViewData> {
    pub fn new(node: VNode<ViewData>) -> Self {
        Self(node)
    }
}

trait VComponentConstruct: Debug {
    type ViewData: VViewData;

    fn props(&self) -> &dyn Any;
    fn construct(&mut self, component: &mut VComponentHead<Self::ViewData>) -> VComponentBody<Self::ViewData>;

    fn run_effects(&mut self, component: &mut VComponentHead<Self::ViewData>);
    fn run_update_destructors(&mut self, component: &mut VComponentHead<Self::ViewData>);
    fn run_permanent_destructors(&mut self, component: &mut VComponentHead<Self::ViewData>);
}

impl <ViewData: VViewData> dyn VComponentConstruct<ViewData=ViewData> {
    #[allow(clippy::needless_lifetimes)]
    pub(super) fn cast_props<'a, Props: Any>(&'a self) -> &'a Props {
        self.props().downcast_ref().expect("props casted to the wrong type")
    }
}

struct VComponentConstructImpl<Props: Any, ViewData: VViewData, F: Fn(VComponentContext2<'_, Props, ViewData>) -> VComponentBody<ViewData> + 'static> {
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

impl <Props: Any, ViewData: VViewData, F: Fn(VComponentContext2<'_, Props, ViewData>) -> VComponentBody<ViewData> + 'static> VComponentConstruct for VComponentConstructImpl<Props, ViewData, F> {
    type ViewData = ViewData;

    fn props(&self) -> &dyn Any {
        &self.props
    }

    fn construct(&mut self, component: &mut VComponentHead<Self::ViewData>) -> VComponentBody<Self::ViewData> {
        let construct = &self.construct;
        construct((VComponentContext1 {
            component,
            effects: &mut self.effects,
        }, &self.props))
    }

    fn run_effects(&mut self, component: &mut VComponentHead<Self::ViewData>) {
        while let Some(effect) = self.effects.effects.pop() {
            if component.has_pending_updates {
                break
            }

            effect((VEffectContext1 {
                component,
                destructors: &mut self.destructors
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

#[derive(Clone)]
pub struct VComponentRef<ViewData: VViewData> {
    renderer: Weak<dyn VComponentRoot<ViewData = ViewData>>,
    path: VComponentPath
}

pub struct VComponent<ViewData: VViewData> {
    pub head: VComponentHead<ViewData>,
    pub(super) construct: Box<dyn VComponentConstruct<ViewData = ViewData>>,
}

#[derive(Debug)]
struct RecursiveUpdateStack(Vec<RecursiveUpdateFrame>);

impl RecursiveUpdateStack {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    fn last_open(&mut self) -> Option<&mut RecursiveUpdateFrame> {
        self.0.last_mut().filter(|frame| frame.is_open)
    }

    fn last_open_or_make(&mut self) -> &mut RecursiveUpdateFrame {
        if !self.0.last_mut().is_some_and(|frame| frame.is_open) {
            self.0.push(RecursiveUpdateFrame::new())
        }
        self.last_open().unwrap()
    }
    
    pub fn add_to_last(&mut self, name: Cow<'static, str>) {
        self.last_open_or_make().add(name);
    }
    
    pub fn close_last(&mut self) {
        if let Some(last) = self.last_open() {
            last.close();
        }
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl Display for RecursiveUpdateStack {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for frame in self.0.iter() {
            write!(f, "{}\n", frame)?;
        }
        write!(f, "---")?;
        Ok(())
    }
}

#[derive(Debug)]
struct RecursiveUpdateFrame {
    simultaneous: Vec<Cow<'static, str>>,
    is_open: bool
}

impl RecursiveUpdateFrame {
    pub fn new() -> Self {
        Self {
            simultaneous: Vec::new(),
            is_open: true
        }
    }

    pub fn is_open(&self) -> bool {
        self.is_open
    }

    pub fn close(&mut self) {
        assert!(self.is_open, "already closed");
        self.is_open = false;
    }

    pub fn add(&mut self, name: Cow<'static, str>) {
        assert!(self.is_open, "closed");
        self.simultaneous.push(name);
    }
}

impl Display for RecursiveUpdateFrame {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.simultaneous.join(", "))
    }
}

pub struct VComponentHead<ViewData: VViewData> {
    /* readonly pub */  id: NodeId,
    /* readonly pub */  key: VComponentKey,
    /* readonly pub */  parent_path: VComponentPath,

    node: Option<VComponentBody<ViewData>>,

    /*readonly*/ children: HashMap<VComponentKey, Box<VComponent<ViewData>>>,
    /*readonly*/ renderer: Weak<dyn VComponentRoot<ViewData = ViewData>>,

    pub(in crate::core) h: VComponentStateData,

    is_being_updated: bool,
    is_fresh: bool,
    has_pending_updates: bool,
    recursive_update_stack_trace: RecursiveUpdateStack,
}

pub(in crate::core) struct VComponentStateData {
    pub state: Vec<Box<dyn Any>>,
    pub next_state_index: usize,
    // pub(in crate::core) provided_contexts: HashMap<Context, Box<dyn Any>>,
    // pub(in crate::core) consumed_contexts: HashMap<Context, Box<dyn Any>>
}

impl <ViewData: VViewData + 'static> VComponent<ViewData> {
    pub(in crate::core) fn new<Props: 'static, F: Fn(VComponentContext2<'_, Props, ViewData>) -> VComponentBody<ViewData> + 'static>(parent: VParent<'_, ViewData>, key: VComponentKey, props: Props, construct: F) -> Box<VComponent<ViewData>> {
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

    fn create<Props: 'static, F: Fn(VComponentContext2<'_, Props, ViewData>) -> VComponentBody<ViewData> + 'static>(parent: VParent<'_, ViewData>, key: VComponentKey, props: Props, construct: F) -> Box<Self>{
        Box::new(VComponent {
            head: VComponentHead {
                id: VNode::<ViewData>::next_id(),
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
                has_pending_updates: false,
                recursive_update_stack_trace: RecursiveUpdateStack::new(),
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

impl <ViewData: VViewData> VComponentHead<ViewData> {
    pub(in crate::core) fn update(&mut self, details: Cow<'static, str>) {
        self.has_pending_updates = true;
        if VMode::is_debug() {
            self.recursive_update_stack_trace.add_to_last(details);
        }
    }

    fn invalidate(&self) {
        if let Some(renderer) = self.renderer.upgrade() {
            renderer.invalidate(self.view());
        }
    }

    pub(in crate::core) fn invalidate_flag(&self) -> WeakArc<NotifyFlag> {
        match self.renderer.upgrade() {
            None => WeakArc::new(),
            Some(renderer) => renderer.invalidate_flag_for(self.view())
        }
    }

    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn key(&self) -> VComponentKey {
        self.key.clone()
    }

    pub(super) fn path(&self) -> VComponentPath {
        self.parent_path.clone() + self.key.clone()
    }

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

    pub fn is_being_created(&self) -> bool {
        self.node.is_none()
    }

    #[allow(clippy::needless_lifetimes)]
    pub(super) fn child<'a>(&'a self, key: &VComponentKey) -> Option<&'a VComponentHead<ViewData>> {
        self.children.get(key).map(|component| &component.head)
    }

    #[allow(clippy::needless_lifetimes)]
    pub(super) fn child_mut<'a>(&'a mut self, key: &VComponentKey) -> Option<&'a mut VComponentHead<ViewData>> {
        self.children.get_mut(key).map(|component| &mut component.head)
    }

    #[allow(clippy::needless_lifetimes)]
    pub fn component_and_view<'a>(&'a self) -> VComponentAndView<'a, ViewData> {
        self.node.as_ref().expect("tried to get view of uninitialized component").0.component_and_view(self)
    }

    #[allow(clippy::needless_lifetimes)]
    pub fn view<'a>(&'a self) -> &'a Box<VView<ViewData>> {
        self.node.as_ref().expect("tried to get view of uninitialized component").0.view(self)
    }

    pub(in crate::core) fn renderer(&self) -> Weak<dyn VComponentRoot<ViewData = ViewData>> {
        self.renderer.clone()
    }
}

impl <ViewData: VViewData> VComponent<ViewData> {
    pub(in crate::core) fn update(mut self: &mut Box<Self>) {
        while self.head.has_pending_updates {
            self.head.has_pending_updates = false;
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
                    node.0.update(self_);

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
                    node.0.update(self_);

                    self_.head.invalidate();
                    self_.head.node = Some(node)
                })
            }
        }
        
        self.head.recursive_update_stack_trace.clear()
    }

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

    fn do_update(mut self: &mut Box<Self>, body: impl FnOnce(&mut Box<Self>) -> ()) {
        self.head.is_being_updated = true;

        body(self);

        self.clear_fresh_and_remove_stale_children();
        self.head.is_being_updated = false;
        self.run_effects();
    }

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

    fn run_effects(self: &mut Box<Self>) {
        self.construct.run_effects(&mut self.head);
    }

    fn run_update_destructors(self: &mut Box<Self>) {
        self.construct.run_update_destructors(&mut self.head);
    }

    fn run_permanent_destructors(self: &mut Box<Self>) {
        self.construct.run_permanent_destructors(&mut self.head);
    }

    pub(super) fn child<'a>(self: &'a Box<Self>, key: &VComponentKey) -> Option<&'a Box<VComponent<ViewData>>> {
        self.head.children.get(key)
    }

    pub(super) fn child_mut<'a>(self: &'a mut Box<Self>, key: &VComponentKey) -> Option<&'a mut Box<VComponent<ViewData>>> {
        self.head.children.get_mut(key)
    }

    pub(in crate::core) fn down_path<'a>(self: &'a Box<Self>, path: &'a VComponentPath) -> Option<&Box<VComponent<ViewData>>> {
        let mut current = self;
        for segment in path.iter() {
            current = current.child(segment)?;
        }
        Some(current)
    }

    pub(in crate::core) fn down_path_mut<'a>(self: &'a mut Box<Self>, path: &'a VComponentPath) -> Option<&mut Box<VComponent<ViewData>>> {
        let mut current = self;
        for segment in path.iter() {
            current = current.child_mut(segment)?;
        }
        Some(current)
    }
}

impl <ViewData: VViewData> VComponentRef<ViewData> {
    pub fn with<R>(&self, fun: impl FnOnce(Option<&mut Box<VComponent<ViewData>>>) -> R) -> R {
        match self.renderer.upgrade() {
            None => fun(None),
            Some(renderer) => {
                // We can't return values in renderer's `with` because it's a trait object
                let mut return_value: MaybeUninit<R> = MaybeUninit::uninit();
                renderer.with_component(&self.path, |component| {
                    return_value.write(fun(component));
                });
                unsafe { return_value.assume_init() }
            }
        }
    }

    pub fn try_with<R>(&self, fun: impl FnOnce(&mut Box<VComponent<ViewData>>) -> R) -> Option<R> {
        self.with(|component| {
            component.map(fun)
        })
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

impl <Props: Any, ViewData: VViewData, F: Fn(VComponentContext2<'_, Props, ViewData>) -> VComponentBody<ViewData> + 'static> Debug for VComponentConstructImpl<Props, ViewData, F> {
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

impl <ViewData: VViewData + Debug> Debug for VComponent<ViewData> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VComponent")
            .field("head", &self.head)
            .field("construct", &self.construct)
            .finish()
    }
}


impl <ViewData: VViewData> Debug for VComponentRef<ViewData> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("VComponentRef").field(&self.path).finish()
    }
}

impl <ViewData: VViewData> PartialEq for VComponentRef<ViewData> {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }

    fn ne(&self, other: &Self) -> bool {
        self.path != other.path
    }
}