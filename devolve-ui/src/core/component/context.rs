//! `VContext` is passed as an argument to component functions, effects, and basically any other
//! scope within components. `VContext` contains the props and state, and the `VContext` passed to
//! component functions also allows you to register hooks. You can't just transfer `VContext`s wherever
//! you want as they contain references, and indeed, they get stale, and the `VContext` within effects
//! is different than that in your component body.
//!
//! `VContext` has no equivalent in React. In react, the component's context is implicit, and you
//! can access props and state in scope without worrying about lifetimes and register hooks wherever
//! you want. But this is bad, as it leads to at best runtime errors and at worst the [stale closure
//! problem](https://stackoverflow.com/questions/62806541/how-to-solve-the-react-hook-closure-issue).
//!
//! (Snark) unlike in JavaScript, in Rust it's common convention to catch errors at compile time
//! instead of throwing runtime exceptions or just returning `undefined`.

use std::any::Any;
use std::iter;
use std::marker::PhantomData;
use crate::core::component::component::{VComponentContexts, VComponentDestructors, VComponentEffects, VComponentHead};
use crate::core::component::path::{VComponentRef, VComponentRefResolved};
use crate::core::hooks::context::AnonContextId;
use crate::core::view::view::VViewData;

#[derive(Debug)]
pub struct VComponentContext1<'a, Props: Any, ViewData: VViewData> {
    pub(in crate::core) component: &'a mut VComponentHead<ViewData>,
    pub(in crate::core) contexts: &'a mut VComponentContexts<'a>,
    pub(in crate::core) effects: &'a mut VComponentEffects<Props, ViewData>,
    // This doesn't need to be PhantomData but it needs to be private so crate::core can't construct this
    pub(super) phantom: PhantomData<Props>
}

pub struct VEffectContext1<'a, Props: Any, ViewData: VViewData> {
    pub(in crate::core) component: &'a mut VComponentHead<ViewData>,
    pub(in crate::core) contexts: &'a mut VComponentContexts<'a>,
    pub(in crate::core) destructors: &'a mut VComponentDestructors<Props, ViewData>,
    // This doesn't need to be PhantomData but it needs to be private so crate::core can't construct this
    pub(super) phantom: PhantomData<Props>
}

#[derive(Debug)]
pub struct VDestructorContext1<'a, Props: Any, ViewData: VViewData> {
    pub component: &'a mut VComponentHead<ViewData>,
    pub(in crate::core) contexts: &'a mut VComponentContexts<'a>,
    // This needs to be private so users can't construct this even though all other fields are public
    pub(super) phantom: PhantomData<Props>
}

#[derive(Debug)]
pub struct VPlainContext1<'a, Props: Any, ViewData: VViewData> {
    pub(super) component: &'a mut VComponentHead<ViewData>,
    pub(super) contexts: &'a mut VComponentContexts<'a>,
    // This just needs to be PhantomData for Props
    pub(super) phantom: PhantomData<Props>
}

#[derive(Debug, Clone)]
pub struct VEffectContextRef<Props: Any, ViewData: VViewData> {
    component: VComponentRef<ViewData>,
    phantom: PhantomData<Props>
}

pub type VComponentContext2<'a, Props, ViewData> = (VComponentContext1<'a, Props, ViewData>, &'a Props);

pub type VEffectContext2<'a, Props, ViewData> = (VEffectContext1<'a, Props, ViewData>, &'a Props);

pub type VDestructorContext2<'a, Props, ViewData> = (VDestructorContext1<'a, Props, ViewData>, &'a Props);

pub type VPlainContext2<'a, Props, ViewData> = (VPlainContext1<'a, Props, ViewData>, &'a Props);

pub trait VContext<'a> {
    type ViewData: VViewData;

    fn component<'b>(&'b mut self) -> &'b mut VComponentHead<Self::ViewData> where 'a: 'b;
    fn get_mut_context<'b>(&'b mut self, id: &AnonContextId) -> Option<&'b mut Box<dyn Any>> where 'a: 'b;
}

pub trait VComponentContext<'a> : VContext<'a> {
    fn insert_mut_context<'b>(&'b mut self, id: AnonContextId, value: Box<dyn Any>) -> Option<Box<dyn Any>> where 'a: 'b;
}

impl <'a, Props: Any, ViewData: VViewData> VContext<'a> for VComponentContext1<'a, Props, ViewData> {
    type ViewData = ViewData;

    fn component<'b>(&'b mut self) -> &'b mut VComponentHead<Self::ViewData> where 'a: 'b {
        self.component
    }

    fn get_mut_context<'b>(&'b mut self, id: &AnonContextId) -> Option<&'b mut Box<dyn Any>> where 'a: 'b {
        self.contexts.get_mut(id)
    }
}

impl <'a, Props: Any, ViewData: VViewData> VComponentContext<'a> for VComponentContext1<'a, Props, ViewData> {
    fn insert_mut_context<'b>(&'b mut self, id: AnonContextId, value: Box<dyn Any>) -> Option<Box<dyn Any>> where 'a: 'b {
        self.contexts.top_mut().expect("empty context stack in hook").insert(id, value)
    }
}

impl <'a, Props: Any, ViewData: VViewData> VContext<'a> for VEffectContext1<'a, Props, ViewData> {
    type ViewData = ViewData;

    fn component<'b>(&'b mut self) -> &'b mut VComponentHead<Self::ViewData> where 'a: 'b {
        self.component
    }

    fn get_mut_context<'b>(&'b mut self, id: &AnonContextId) -> Option<&'b mut Box<dyn Any>> where 'a: 'b {
        self.contexts.get_mut(id)
    }
}

impl <'a, Props: Any, ViewData: VViewData> VEffectContext1<'a, Props, ViewData> {
    pub(in crate::core) fn component_and_destructors<'b>(&'b mut self) -> (&'b mut VComponentHead<ViewData>, &'b mut VComponentDestructors<Props, ViewData>) where 'a: 'b {
        (self.component, self.destructors)
    }

    pub(in crate::core) fn destructors<'b>(&'b mut self) -> &'b mut VComponentDestructors<Props, ViewData> where 'a: 'b {
        self.destructors
    }

    /// Reference to the `VComponent` and associated data which can be cloned and lifetime extended.
    /// When you want to get the `VEffectContext` back you can call `with`.
    /// This allows you to transfer effect context data (e.g. props) across time and threads.
    ///
    /// **Warning:** Calling `with` on multiple components at the same time (e.g. nested) will cause a runtime error.
    pub fn vref(&mut self) -> VEffectContextRef<Props, ViewData> {
        VEffectContextRef {
            component: self.component.vref(),
            phantom: PhantomData
        }
    }

    pub fn with<'b, R>(&'b mut self, fun: impl FnOnce(VEffectContext1<'b, Props, ViewData>) -> R) -> R {
        fun(VEffectContext1 {
            component: self.component,
            contexts: self.contexts,
            destructors: self.destructors,
            phantom: PhantomData
        })
    }
}

impl <Props: Any, ViewData: VViewData + 'static> VEffectContextRef<Props, ViewData> {
    pub fn with<R>(&self, fun: impl FnOnce(Option<VPlainContext2<'_, Props, ViewData>>) -> R) -> R {
        self.component.with(|component| {
            match component {
                None => fun(None),
                Some(VComponentRefResolved { parent_contexts, component }) => fun(Some((VPlainContext1 {
                    component: &mut component.head,
                    contexts: parent_contexts.into_iter().chain(component.construct.local_contexts()).collect(),
                    phantom: PhantomData
                }, component.construct.cast_props())))
            }
        })
    }

    pub fn try_with<R>(&self, fun: impl FnOnce(VPlainContext2<'_, Props, ViewData>) -> R) -> Option<R> {
        self.component.try_with(|VComponentRefResolved { parent_contexts, component }| {
            fun((VPlainContext1 {
                component: &mut component.head,
                contexts: parent_contexts.into_iter().chain(component.construct.local_contexts()).collect(),
                phantom: PhantomData
            }, component.construct.cast_props()))
        })
    }
}

impl <'a, Props: Any, ViewData: VViewData> VContext<'a> for VDestructorContext1<'a, Props, ViewData> {
    type ViewData = ViewData;

    fn component<'b>(&'b mut self) -> &'b mut VComponentHead<Self::ViewData> where 'a: 'b {
        self.component
    }

    fn get_mut_context<'b>(&'b mut self, id: &AnonContextId) -> Option<&'b mut Box<dyn Any>> where 'a: 'b {
        self.contexts.get_mut(id)
    }
}

impl <'a, Props: Any, ViewData: VViewData> VDestructorContext1<'a, Props, ViewData> {
    pub fn with<'b, R>(&'b mut self, fun: impl FnOnce(VDestructorContext1<'b, Props, ViewData>) -> R) -> R {
        fun((VDestructorContext1 {
            component: self.component,
            contexts: self.contexts,
            phantom: PhantomData
        }))
    }
}

impl <'a, Props: Any, ViewData: VViewData> VContext<'a> for VPlainContext1<'a, Props, ViewData> {
    type ViewData = ViewData;

    fn component<'b>(&'b mut self) -> &'b mut VComponentHead<Self::ViewData> where 'a: 'b {
        self.component
    }

    fn get_mut_context<'b>(&'b mut self, id: &AnonContextId) -> Option<&mut Box<dyn Any>> {
        self.contexts.get_mut(id)
    }
}

impl <'a, Props: Any, ViewData: VViewData> VPlainContext1<'a, Props, ViewData> {
    pub fn with<'b, R>(&'b mut self, fun: impl FnOnce(VPlainContext1<'b, Props, ViewData>) -> R) -> R {
        fun((VPlainContext1 {
            component: self.component,
            contexts: self.contexts,
            phantom: PhantomData
        }))
    }
}

pub fn with_destructor_context<'a, Props: Any, ViewData: VViewData, R>(
    (c, props): (&mut VEffectContext1<'a, Props, ViewData>, &'a Props),
    fun: impl FnOnce(VDestructorContext2<'_, Props, ViewData>) -> R
) -> R {
    fun((VDestructorContext1 {
        component: c.component,
        contexts: c.contexts,
        phantom: PhantomData
    }, props))
}

pub fn with_plain_context<'a, Props: Any, ViewData: VViewData, R>(
    (c, props): (&mut VEffectContext1<'a, Props, ViewData>, &'a Props),
    fun: impl FnOnce(VPlainContext2<'_, Props, ViewData>) -> R
) -> R {
    fun((VPlainContext1 {
        component: c.component,
        contexts: c.contexts,
        phantom: PhantomData
    }, props))
}