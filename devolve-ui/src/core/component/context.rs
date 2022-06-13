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
use crate::core::component::component::{VComponentEffects, VComponentHead};
use crate::core::view::view::VViewData;

#[derive(Debug)]
pub struct VComponentContextImpl<'a, Props: Any, ViewData: VViewData> {
    pub(super) component: &'a mut VComponentHead<ViewData>,
    pub(super) props: &'a Props,
    pub(in crate::core) effects: &'a mut VComponentEffects<Props, ViewData>
}

#[derive(Debug)]
pub struct VEffectContextImpl<'a, Props: Any, ViewData: VViewData> {
    pub(super) component: &'a mut VComponentHead<ViewData>,
    pub(super) props: &'a Props,
}

pub trait VContext<'a> {
    type ViewData: VViewData;

    fn component(&'a mut self) -> &'a mut VComponentHead<Self::ViewData>;
}

impl <'a, Props: Any, ViewData: VViewData> VContext<'a> for VComponentContextImpl<'a, Props, ViewData> {
    type ViewData = ViewData;

    fn component(&'a mut self) -> &'a mut VComponentHead<Self::ViewData> {
        self.component
    }
}

impl <'a, Props: Any, ViewData: VViewData> VComponentContextImpl<'a, Props, ViewData> {
    pub fn props(&'a self) -> &'a Props {
        self.props
    }

    pub fn into_effect_ctx(self) -> VEffectContextImpl<'a, Props, ViewData> {
        Self::EffectContext {
            component: self.component,
            props: self.props
        }
    }
}

impl <'a, Props: Any, ViewData: VViewData> VContext<'a> for VEffectContextImpl<'a, Props, ViewData> {
    type ViewData = ViewData;

    fn component(&'a mut self) -> &'a mut VComponentHead<Self::ViewData> {
        self.component
    }
}

impl <'a, Props: Any, ViewData: VViewData> VEffectContextImpl<'a, Props, ViewData> {
    pub fn props(&'a self) -> &'a Props {
        self.props
    }
}