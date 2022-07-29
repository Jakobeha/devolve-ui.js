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
use std::iter::once;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};
use crate::component::component::{ContextPendingUpdates, VComponentContexts, VComponentDestructors, VComponentEffects, VComponentHead, VComponentLocalContexts};
use crate::component::path::{VComponentRef, VComponentRefResolved};
use crate::hooks::provider::UntypedProviderId;
use crate::view::view::VViewData;

#[derive(Debug)]
pub struct VComponentContext1<'a, 'a0: 'a, Props: Any, ViewData: VViewData> {
    pub(crate) component: &'a mut VComponentHead<ViewData>,
    pub(crate) contexts: &'a mut VComponentContexts<'a0>,
    pub(crate) effects: &'a mut VComponentEffects<Props, ViewData>,
    // This doesn't need to be PhantomData but it needs to be private so crate::core can't construct this
    pub(super) phantom: PhantomData<Props>
}

#[derive(Debug, Clone, Copy)]
pub struct VComponentContextUnsafe {
    pub component: *mut (),
    pub contexts: *mut (),
    pub effects: *mut (),
}

impl<'a, 'a0: 'a, Props: Any, ViewData: VViewData> VComponentContext1<'a, 'a0, Props, ViewData> {
    pub unsafe fn from_unsafe(ptr: VComponentContextUnsafe) -> Self {
        VComponentContext1 {
            component: &mut *(ptr.component as *mut VComponentHead<ViewData>),
            contexts: &mut *(ptr.contexts as *mut VComponentContexts<'a0>),
            effects: &mut *(ptr.effects as *mut VComponentEffects<Props, ViewData>),
            phantom: PhantomData
        }
    }

    pub fn as_unsafe(&mut self) -> VComponentContextUnsafe {
        VComponentContextUnsafe {
            component: self.component as *mut _ as *mut (),
            contexts: self.contexts as *mut _ as *mut (),
            effects: self.effects as *mut _ as *mut ()
        }
    }
}

pub struct VEffectContext1<'a, 'a0: 'a, Props: Any, ViewData: VViewData> {
    pub(crate) component: &'a mut VComponentHead<ViewData>,
    pub(crate) contexts: &'a mut VComponentContexts<'a0>,
    pub(crate) destructors: &'a mut VComponentDestructors<Props, ViewData>,
    // This doesn't need to be PhantomData but it needs to be private so crate::core can't construct this
    pub(super) phantom: PhantomData<Props>
}

#[derive(Debug)]
pub struct VDestructorContext1<'a, 'a0: 'a, Props: Any, ViewData: VViewData> {
    pub component: &'a mut VComponentHead<ViewData>,
    pub(crate) contexts: &'a mut VComponentContexts<'a0>,
    // This needs to be private so users can't construct this even though all other fields are public
    pub(super) phantom: PhantomData<Props>
}

#[derive(Debug)]
pub struct VPlainContext1<'a, 'a0: 'a, Props: Any, ViewData: VViewData> {
    pub(super) component: &'a mut VComponentHead<ViewData>,
    pub(super) contexts: &'a mut VComponentContexts<'a0>,
    // This just needs to be PhantomData for Props
    pub(super) phantom: PhantomData<Props>
}

#[derive(Debug, Clone)]
pub struct VEffectContextRef<Props: Any, ViewData: VViewData> {
    component: VComponentRef<ViewData>,
    phantom: PhantomData<Props>
}

pub type VComponentContext2<'a, 'a0, Props, ViewData> = (VComponentContext1<'a, 'a0, Props, ViewData>, &'a Props);

pub type VEffectContext2<'a, 'a0, Props, ViewData> = (VEffectContext1<'a, 'a0, Props, ViewData>, &'a Props);

pub type VDestructorContext2<'a, 'a0, Props, ViewData> = (VDestructorContext1<'a, 'a0, Props, ViewData>, &'a Props);

pub type VPlainContext2<'a, 'a0, Props, ViewData> = (VPlainContext1<'a, 'a0, Props, ViewData>, &'a Props);

pub trait VContext<'a> {
    type ViewData: VViewData;

    fn component_imm<'b>(&'b self) -> &'b VComponentHead<Self::ViewData> where 'a: 'b;
    fn component<'b>(&'b mut self) -> &'b mut VComponentHead<Self::ViewData> where 'a: 'b;
    fn get_context<'b>(&'b self, id: &UntypedProviderId) -> Option<&'b Box<dyn Any>> where 'a: 'b;
    fn get_mut_context<'b>(&'b mut self, id: &UntypedProviderId) -> Option<(&'b mut Box<dyn Any>, &'b mut ContextPendingUpdates)> where 'a: 'b;
}

pub trait VComponentContext<'a, 'a0> : VContext<'a> {
    fn component_and_contexts<'b>(&'b mut self) -> (&'b mut VComponentHead<Self::ViewData>, &'b mut VComponentContexts<'a0>) where 'a: 'b;
    fn local_contexts<'b>(&'b mut self) -> &'b mut VComponentLocalContexts where 'a: 'b;
}

impl <'a, 'a0: 'a, Props: Any, ViewData: VViewData> VContext<'a> for VComponentContext1<'a, 'a0, Props, ViewData> {
    type ViewData = ViewData;

    fn component_imm<'b>(&'b self) -> &'b VComponentHead<Self::ViewData> where 'a: 'b {
        self.component
    }

    fn component<'b>(&'b mut self) -> &'b mut VComponentHead<Self::ViewData> where 'a: 'b {
        self.component
    }

    fn get_context<'b>(&'b self, id: &UntypedProviderId) -> Option<&'b Box<dyn Any>> where 'a: 'b {
        self.contexts.get(id).map(|(context, _path)| context)
    }

    fn get_mut_context<'b>(&'b mut self, id: &UntypedProviderId) -> Option<(&'b mut Box<dyn Any>, &'b mut ContextPendingUpdates)> where 'a: 'b {
        self.contexts.get_mut(id)
    }
}

impl <'a, 'a0: 'a, Props: Any, ViewData: VViewData> VComponentContext<'a, 'a0> for VComponentContext1<'a, 'a0, Props, ViewData> {
    fn component_and_contexts<'b>(&'b mut self) -> (&'b mut VComponentHead<Self::ViewData>, &'b mut VComponentContexts<'a0>) where 'a: 'b {
        (self.component, self.contexts)
    }

    fn local_contexts<'b>(&'b mut self) -> &'b mut VComponentLocalContexts where 'a: 'b {
        self.contexts.top_mut().expect("empty context stack in hook").0
    }
}

impl <'a, 'a0: 'a, Props: Any, ViewData: VViewData> VContext<'a> for VEffectContext1<'a, 'a0, Props, ViewData> {
    type ViewData = ViewData;

    fn component_imm<'b>(&'b self) -> &'b VComponentHead<Self::ViewData> where 'a: 'b {
        self.component
    }

    fn component<'b>(&'b mut self) -> &'b mut VComponentHead<Self::ViewData> where 'a: 'b {
        self.component
    }

    fn get_context<'b>(&'b self, id: &UntypedProviderId) -> Option<&'b Box<dyn Any>> where 'a: 'b {
        self.contexts.get(id).map(|(context, _path)| context)
    }

    fn get_mut_context<'b>(&'b mut self, id: &UntypedProviderId) -> Option<(&'b mut Box<dyn Any>, &'b mut ContextPendingUpdates)> where 'a: 'b {
        self.contexts.get_mut(id)
    }
}

impl <'a, 'a0: 'a, Props: Any, ViewData: VViewData> VEffectContext1<'a, 'a0, Props, ViewData> {
    pub(crate) fn component_and_destructors<'b>(&'b mut self) -> (&'b mut VComponentHead<ViewData>, &'b mut VComponentDestructors<Props, ViewData>) where 'a: 'b {
        (self.component, self.destructors)
    }

    pub(crate) fn destructors<'b>(&'b mut self) -> &'b mut VComponentDestructors<Props, ViewData> where 'a: 'b {
        self.destructors
    }

    /// Reference to the `VComponent` and associated data which can be cloned and lifetime extended.
    /// When you want to get the `VEffectContext` back you can call `with`.
    /// This allows you to transfer effect context data (e.g. props) across time and threads.
    ///
    /// **Warning:** Calling `with` on multiple components at the same time (e.g. nested) will cause a runtime error.
    ///
    /// TODO: Improve and make actually `Send`
    pub fn vref(&mut self) -> VEffectContextRef<Props, ViewData> {
        VEffectContextRef {
            component: self.component.vref(),
            phantom: PhantomData
        }
    }

    pub fn with<'b, R>(&'b mut self, fun: impl FnOnce(VEffectContext1<'b, 'a0, Props, ViewData>) -> R) -> R {
        fun(VEffectContext1 {
            component: self.component,
            contexts: self.contexts,
            destructors: self.destructors,
            phantom: PhantomData
        })
    }
}

impl <Props: Any, ViewData: VViewData + 'static> VEffectContextRef<Props, ViewData> {
    pub fn with<R>(&self, fun: impl FnOnce(Option<VPlainContext2<'_, '_, Props, ViewData>>) -> R) -> R {
        self.component.with(|component| {
            match component {
                None => fun(None),
                Some(VComponentRefResolved { parent_contexts, component }) => {
                    let (local_contexts, local_context_changes, props) = component.construct.local_contexts_and_cast_props();
                    fun(Some((VPlainContext1 {
                        component: &mut component.head,
                        contexts: &mut parent_contexts.into_iter().chain(once((local_contexts, local_context_changes))).collect(),
                        phantom: PhantomData
                    }, props)))
                }
            }
        })
    }

    pub fn try_with<R>(&self, fun: impl FnOnce(VPlainContext2<'_, '_, Props, ViewData>) -> R) -> Option<R> {
        self.component.try_with(|VComponentRefResolved { parent_contexts, component }| {
            let (local_contexts, local_context_changes, props) = component.construct.local_contexts_and_cast_props();
            fun((VPlainContext1 {
                component: &mut component.head,
                contexts: &mut parent_contexts.into_iter().chain(once((local_contexts, local_context_changes))).collect(),
                phantom: PhantomData
            }, props))
        })
    }
}

impl <'a, 'a0: 'a, Props: Any, ViewData: VViewData> VContext<'a> for VDestructorContext1<'a, 'a0, Props, ViewData> {
    type ViewData = ViewData;

    fn component_imm<'b>(&'b self) -> &'b VComponentHead<Self::ViewData> where 'a: 'b {
        self.component
    }

    fn component<'b>(&'b mut self) -> &'b mut VComponentHead<Self::ViewData> where 'a: 'b {
        self.component
    }

    fn get_context<'b>(&'b self, id: &UntypedProviderId) -> Option<&'b Box<dyn Any>> where 'a: 'b {
        self.contexts.get(id).map(|(context, _path)| context)
    }

    fn get_mut_context<'b>(&'b mut self, id: &UntypedProviderId) -> Option<(&'b mut Box<dyn Any>, &'b mut ContextPendingUpdates)> where 'a: 'b {
        self.contexts.get_mut(id)
    }
}

impl <'a, 'a0: 'a, Props: Any, ViewData: VViewData> VDestructorContext1<'a, 'a0, Props, ViewData> {
    pub fn with<'b, R>(&'b mut self, fun: impl FnOnce(VDestructorContext1<'b, 'a0, Props, ViewData>) -> R) -> R {
        fun(VDestructorContext1 {
            component: self.component,
            contexts: self.contexts,
            phantom: PhantomData
        })
    }
}

impl <'a, 'a0: 'a, Props: Any, ViewData: VViewData> VContext<'a> for VPlainContext1<'a, 'a0, Props, ViewData> {
    type ViewData = ViewData;

    fn component_imm<'b>(&'b self) -> &'b VComponentHead<Self::ViewData> where 'a: 'b {
        self.component
    }

    fn component<'b>(&'b mut self) -> &'b mut VComponentHead<Self::ViewData> where 'a: 'b {
        self.component
    }

    fn get_context<'b>(&'b self, id: &UntypedProviderId) -> Option<&'b Box<dyn Any>> where 'a: 'b {
        self.contexts.get(id).map(|(context, _path)| context)
    }

    fn get_mut_context<'b>(&'b mut self, id: &UntypedProviderId) -> Option<(&'b mut Box<dyn Any>, &'b mut ContextPendingUpdates)> where 'a: 'b {
        self.contexts.get_mut(id)
    }
}

impl <'a, 'a0: 'a, Props: Any, ViewData: VViewData> VPlainContext1<'a, 'a0, Props, ViewData> {
    pub fn with<'b, R>(&'b mut self, fun: impl FnOnce(VPlainContext1<'b, 'a0, Props, ViewData>) -> R) -> R {
        fun(VPlainContext1 {
            component: self.component,
            contexts: self.contexts,
            phantom: PhantomData
        })
    }
}

pub fn with_destructor_context<'a, 'a0: 'a, Props: Any, ViewData: VViewData, R>(
    (c, props): (&mut VEffectContext1<'a, 'a0, Props, ViewData>, &'a Props),
    fun: impl FnOnce(VDestructorContext2<'_, '_, Props, ViewData>) -> R
) -> R {
    fun((VDestructorContext1 {
        component: c.component,
        contexts: c.contexts,
        phantom: PhantomData
    }, props))
}

pub fn with_plain_context<'a, 'a0: 'a, Props: Any, ViewData: VViewData, R>(
    (c, props): (&mut VEffectContext1<'a, 'a0, Props, ViewData>, &'a Props),
    fun: impl FnOnce(VPlainContext2<'_, '_, Props, ViewData>) -> R
) -> R {
    fun((VPlainContext1 {
        component: c.component,
        contexts: c.contexts,
        phantom: PhantomData
    }, props))
}

// region index states
pub trait VContextIndex<ViewData: VViewData> {
    type T: Any;

    fn get<'a: 'b, 'b>(&self, c: &'b impl VContext<'a, ViewData=ViewData>) -> &'b Self::T where ViewData: 'b;
    fn get_mut<'a: 'b, 'b>(&self, c: &'b mut impl VContext<'a, ViewData=ViewData>) -> &'b mut Self::T where ViewData: 'b;
}

impl <'a, 'a0: 'a, Props: Any, ViewData: VViewData, I: VContextIndex<ViewData>> Index<I> for VComponentContext1<'a, 'a0, Props, ViewData> {
    type Output = I::T;

    fn index(&self, index: I) -> &Self::Output {
        index.get(self)
    }
}

impl <'a, 'a0: 'a, Props: Any, ViewData: VViewData, I: VContextIndex<ViewData>> Index<I> for VEffectContext1<'a, 'a0, Props, ViewData> {
    type Output = I::T;

    fn index(&self, index: I) -> &Self::Output {
        index.get(self)
    }
}

impl <'a, 'a0: 'a, Props: Any, ViewData: VViewData, I: VContextIndex<ViewData>> Index<I> for VDestructorContext1<'a, 'a0, Props, ViewData> {
    type Output = I::T;

    fn index(&self, index: I) -> &Self::Output {
        index.get(self)
    }
}

impl <'a, 'a0: 'a, Props: Any, ViewData: VViewData, I: VContextIndex<ViewData>> Index<I> for VPlainContext1<'a, 'a0, Props, ViewData> {
    type Output = I::T;

    fn index(&self, index: I) -> &Self::Output {
        index.get(self)
    }
}

impl <'a, 'a0: 'a, Props: Any, ViewData: VViewData, I: VContextIndex<ViewData>> IndexMut<I> for VComponentContext1<'a, 'a0, Props, ViewData> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        index.get_mut(self)
    }
}

impl <'a, 'a0: 'a, Props: Any, ViewData: VViewData, I: VContextIndex<ViewData>> IndexMut<I> for VEffectContext1<'a, 'a0, Props, ViewData> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        index.get_mut(self)
    }
}

impl <'a, 'a0: 'a, Props: Any, ViewData: VViewData, I: VContextIndex<ViewData>> IndexMut<I> for VDestructorContext1<'a, 'a0, Props, ViewData> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        index.get_mut(self)
    }
}

impl <'a, 'a0: 'a, Props: Any, ViewData: VViewData, I: VContextIndex<ViewData>> IndexMut<I> for VPlainContext1<'a, 'a0, Props, ViewData> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        index.get_mut(self)
    }
}
// endregion