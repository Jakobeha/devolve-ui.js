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
use std::marker::PhantomData;
use crate::core::component::component::{VComponentDestructors, VComponentEffects, VComponentHead, VComponentRef};
use crate::core::view::view::VViewData;

#[derive(Debug)]
pub struct VComponentContext1<'a, Props: Any, ViewData: VViewData> {
    pub(super) component: &'a mut VComponentHead<ViewData>,
    pub(super) effects: &'a mut VComponentEffects<Props, ViewData>
}

pub struct VEffectContext1<'a, Props: Any, ViewData: VViewData> {
    pub(super) component: &'a mut VComponentHead<ViewData>,
    pub(super) destructors: &'a mut VComponentDestructors<Props, ViewData>
}

#[derive(Debug)]
pub struct VDestructorContext1<'a, Props: Any, ViewData: VViewData> {
    pub(super) component: &'a mut VComponentHead<ViewData>,
    pub(super) phantom: PhantomData<Props>
}

#[derive(Debug)]
pub struct VPlainContext1<'a, Props: Any, ViewData: VViewData> {
    pub(super) component: &'a mut VComponentHead<ViewData>,
    pub(super) phantom: PhantomData<Props>
}

#[derive(Debug, Clone)]
pub struct VEffectContextRef<Props: Any, ViewData: VViewData> {
    component: VComponentRef<ViewData>,
    phantom: PhantomData<Props>
}

pub type VComponentContext2<'a: 'b, 'b, Props: Any, ViewData: VViewData> = (&'b mut VComponentContext1<'a, Props, ViewData>, &'a Props);

pub type VEffectContext2<'a: 'b, 'b, Props: Any, ViewData: VViewData> = (&'b mut VEffectContext1<'a, Props, ViewData>, &'a Props);

pub type VDestructorContext2<'a: 'b, 'b, Props: Any, ViewData: VViewData> = (&'b mut VDestructorContext1<'a, Props, ViewData>, &'a Props);

pub type VPlainContext2<'a: 'b, 'b, Props: Any, ViewData: VViewData> = (&'b mut VPlainContext1<'a, Props, ViewData>, &'a Props);

pub trait VContext<'a> {
    type ViewData: VViewData;

    fn component<'b>(&'b mut self) -> &'b mut VComponentHead<Self::ViewData> where 'a: 'b;
}

pub trait VComponentContext<'a> : VContext<'a> {}

impl <'a, Props: Any, ViewData: VViewData> VContext<'a> for VComponentContext1<'a, Props, ViewData> {
    type ViewData = ViewData;

    fn component<'b>(&'b mut self) -> &'b mut VComponentHead<Self::ViewData> where 'a: 'b {
        self.component
    }
}

impl <'a, Props: Any, ViewData: VViewData> VComponentContext<'a> for VComponentContext1<'a, Props, ViewData> {}

impl <'a, Props: Any, ViewData: VViewData> VComponentContext1<'a, Props, ViewData> {
    pub(in crate::core) fn component_and_effects(&'a mut self) -> (&'a mut VComponentHead<ViewData>, &'a mut VComponentEffects<Props, ViewData>) {
        (self.component, self.effects)
    }
}

impl <'a, Props: Any, ViewData: VViewData> VContext<'a> for VEffectContext1<'a, Props, ViewData> {
    type ViewData = ViewData;

    fn component<'b>(&'b mut self) -> &'b mut VComponentHead<Self::ViewData> where 'a: 'b {
        self.component
    }
}

impl <'a, Props: Any, ViewData: VViewData> VEffectContext1<'a, Props, ViewData> {
    pub(in crate::core) fn component_and_destructors(&'a mut self) -> (&'a mut VComponentHead<ViewData>, &'a mut VComponentDestructors<Props, ViewData>) {
        (self.component, self.destructors)
    }

    pub(in crate::core) fn destructors(&'a mut self) -> &'a mut VComponentDestructors<Props, ViewData> {
        self.destructors
    }

    /// Reference to the `VComponent` and associated data which can be cloned and lifetime extended.
    /// When you want to get the `VEffectContext` back you can call `with`.
    /// This allows you to transfer effect context data (e.g. props) across time and threads.
    ///
    /// **Warning:** Calling `with` on multiple components at the same time (e.g. nested) will cause a runtime error.
    pub fn vref(&'a mut self) -> VEffectContextRef<Props, ViewData> {
        VEffectContextRef {
            component: self.component.vref(),
            phantom: PhantomData
        }
    }

    pub fn with_shortened<'b, R>(&'b mut self, fun: impl FnOnce(&'b mut VEffectContext1<'a, Props, ViewData>) -> R) -> R where 'a: 'b {
        fun(self)
    }
}

impl <Props: Any, ViewData: VViewData + 'static> VEffectContextRef<Props, ViewData> {
    fn with<R>(&self, fun: impl FnOnce(Option<VPlainContext2<'_, '_, Props, ViewData>>) -> R) -> R {
        self.component.with(|component| {
            match component {
                None => fun(None),
                Some(component) => fun(Some((&mut VPlainContext1 {
                    component: &mut component.head,
                    phantom: PhantomData
                }, component.construct.cast_props())))
            }
        })
    }

    pub fn try_with<R>(&self, fun: impl FnOnce(VPlainContext2<'_, '_, Props, ViewData>) -> R) -> Option<R> {
        self.component.try_with(|component| {
            fun((&mut VPlainContext1 {
                component: &mut component.head,
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
}

impl <'a, Props: Any, ViewData: VViewData> VContext<'a> for VPlainContext1<'a, Props, ViewData> {
    type ViewData = ViewData;

    fn component<'b>(&'b mut self) -> &'b mut VComponentHead<Self::ViewData> where 'a: 'b {
        self.component
    }
}

pub fn with_destructor_context<Props: Any, ViewData: VViewData, R>(
    (c, props): VEffectContext2<'_, '_, Props, ViewData>,
    fun: impl FnOnce(VDestructorContext2<'_, '_, Props, ViewData>) -> R
) -> R {
    fun((&mut VDestructorContext1 {
        component: c.component,
        phantom: PhantomData
    }, props))
}

pub fn with_plain_context<Props: Any, ViewData: VViewData, R>(
    (c, props): VEffectContext2<'_, '_, Props, ViewData>,
    fun: impl FnOnce(VPlainContext2<'_, '_, Props, ViewData>) -> R
) -> R {
    fun((&mut VPlainContext1 {
        component: c.component,
        phantom: PhantomData
    }, props))
}