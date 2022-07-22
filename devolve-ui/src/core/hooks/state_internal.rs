//! A component's state which doesn't trigger a re-render when set.
//!
//! Normally you should not use this, which is why it's exported in a separate module than the other hooks.
//! Originally this was only used internally by other hooks.
//! However we realized there are public use cases for this as well.

use std::any::Any;
use std::marker::PhantomData;
use crate::core::component::context::{VComponentContext, VContext, VContextIndex};
use crate::core::view::view::VViewData;

#[derive(Debug)]
pub struct NonUpdatingState<T: Any, ViewData: VViewData> {
    pub(super) index: usize,
    phantom: PhantomData<(T, ViewData)>
}

pub trait NonUpdatingStateHook<'a, 'a0: 'a, ViewData: VViewData + 'a> {
    fn use_non_updating_state<T: Any>(&mut self, get_initial: impl FnOnce(&mut Self) -> T) -> NonUpdatingState<T, ViewData>;
}

impl <'a, 'a0: 'a, ViewData: VViewData + 'a, Context: VComponentContext<'a, 'a0, ViewData=ViewData>> NonUpdatingStateHook<'a, 'a0, ViewData> for Context {
    fn use_non_updating_state<T: Any>(&mut self, get_initial: impl FnOnce(&mut Self) -> T) -> NonUpdatingState<T, ViewData> {
        _use_non_updating_state(self, get_initial)
    }
}

fn _use_non_updating_state<'a, 'a0: 'a, T: Any, ViewData: VViewData + 'a, Ctx: VComponentContext<'a, 'a0, ViewData=ViewData>>(
    c: &mut Ctx,
    get_initial: impl FnOnce(&mut Ctx) -> T
) -> NonUpdatingState<T, ViewData> {
    let component = c.component();
    let index = component.h.next_state_index;
    component.h.next_state_index += 1;
    if component.is_being_created() {
        if component.h.state.len() != index {
            panic!("unaligned hooks: state length ({}) != state index ({})", component.h.state.len(), index);
        }
        let initial_state = Box::new(get_initial(c));
        let component = c.component();
        if component.h.state.len() != index {
            panic!("you called a state hook in the get_initial of another state hook. This is not allowed, although you also probably don't want to nest hooks in get_initial anyways");
        }
        component.h.state.push(initial_state);
    }

    NonUpdatingState {
        index,
        phantom: PhantomData
    }
}

impl <T: Any, ViewData: VViewData> NonUpdatingState<T, ViewData> {
    pub fn _get_mut<'a>(&self, c_state: &'a mut Vec<Box<dyn Any>>) -> &'a mut T {
        c_state
            .get_mut(self.index).expect("unaligned hooks: state index out of bounds")
            .downcast_mut::<T>().expect("unaligned hooks: state type mismatch")
    }
}

impl <T: Any, ViewData: VViewData> VContextIndex<ViewData> for NonUpdatingState<T, ViewData> {
    type T = T;

    fn get<'a: 'b, 'b>(&self, c: &'b impl VContext<'a, ViewData=ViewData>) -> &'b T where ViewData: 'b {
        c.component_imm().h.state
            .get(self.index).expect("unaligned hooks: state index out of bounds")
            .downcast_ref::<T>().expect("unaligned hooks: state type mismatch")
    }

    fn get_mut<'a: 'b, 'b>(&self, c: &'b mut impl VContext<'a, ViewData=ViewData>) -> &'b mut T where ViewData: 'b {
        self._get_mut(&mut c.component().h.state)
    }
}

impl <T: Any, ViewData: VViewData> Clone for NonUpdatingState<T, ViewData> {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            phantom: self.phantom
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.index = source.index;
        // No-op
        self.phantom = source.phantom;
    }
}

impl <T: Any, ViewData: VViewData> Copy for NonUpdatingState<T, ViewData> {}
