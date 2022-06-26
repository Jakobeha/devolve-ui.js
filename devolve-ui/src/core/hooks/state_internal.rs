//! A component's state which doesn't trigger a re-render when set.
//! This means that it can't be used in the component's render.
//! Instead it's used internally by other hooks.

use std::any::Any;
use std::marker::PhantomData;
use crate::core::component::context::{VComponentContext, VContext};
use crate::core::view::view::VViewData;

#[derive(Debug)]
pub struct NonUpdatingState<T: Any, ViewData: VViewData> {
    pub(super) index: usize,
    phantom: PhantomData<(T, ViewData)>
}

pub trait InternalHooks <'a, 'a0: 'a, ViewData: VViewData + 'a> {
    fn use_non_updating_state<T: Any>(&mut self, initial_state: impl FnOnce() -> T) -> NonUpdatingState<T, ViewData>;
}

impl <'a, 'a0: 'a, ViewData: VViewData + 'a, Context: VComponentContext<'a, 'a0, ViewData=ViewData>> InternalHooks<'a, 'a0, ViewData> for Context {
    fn use_non_updating_state<T: Any>(&mut self, initial_state: impl FnOnce() -> T) -> NonUpdatingState<T, ViewData> {
        _use_non_updating_state(self, initial_state)
    }
}

fn _use_non_updating_state<'a, 'a0: 'a, T: Any, ViewData: VViewData + 'a>(
    c: &mut impl VComponentContext<'a, 'a0, ViewData=ViewData>,
    initial_state: impl FnOnce() -> T
) -> NonUpdatingState<T, ViewData> {
    let c = c.component();
    let index = c.h.next_state_index;
    c.h.next_state_index += 1;
    if c.is_being_created() {
        if c.h.state.len() != index {
            panic!("unaligned hooks: state length ({}) != state index ({})", c.h.state.len(), index);
        }
        c.h.state.push(Box::new(initial_state()));
    }

    NonUpdatingState {
        index,
        phantom: PhantomData
    }
}


impl <T: Any, ViewData: VViewData> NonUpdatingState<T, ViewData> {
    pub fn get<'a: 'b, 'b>(&self, c: &'b impl VContext<'a, ViewData=ViewData>) -> &'b T where ViewData: 'b {
        c.component_imm().h.state
            .get(self.index).expect("unaligned hooks: state index out of bounds")
            .downcast_ref::<T>().expect("unaligned hooks: state type mismatch")
    }

    pub fn get_mut<'a: 'b, 'b>(&self, c: &'b mut impl VContext<'a, ViewData=ViewData>) -> &'b mut T where ViewData: 'b {
        self._get_mut(&mut c.component().h.state)
    }

    pub fn _get_mut<'a>(&self, c_state: &'a mut Vec<Box<dyn Any>>) -> &'a mut T {
        c_state
            .get_mut(self.index).expect("unaligned hooks: state index out of bounds")
            .downcast_mut::<T>().expect("unaligned hooks: state type mismatch")
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
