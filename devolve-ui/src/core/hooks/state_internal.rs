//! A component's state which doesn't trigger a re-render when set.
//! This means that it can't be used in the component's render.
//! Instead it's used internally by other hooks.

use std::any::Any;
use std::marker::PhantomData;
use crate::core::component::component::VComponent;
use crate::core::view::view::VViewData;

#[derive(Debug)]
pub struct NonUpdatingState<T: Any, ViewData: VViewData> {
    pub index: usize,
    pub phantom_view_data: PhantomData<(T, ViewData)>
}

pub fn use_non_updating_state<T: Any, ViewData: VViewData>(c: &mut Box<VComponent<ViewData>>, initial_state: impl FnOnce() -> T) -> NonUpdatingState<T, ViewData> {
    let index = c.next_state_index;
    c.next_state_index += 1;
    if c.is_being_created() {
        if c.state.len() != index {
            panic!("unaligned hooks: state length ({}) != state index ({})", c.state.len(), index);
        }
        c.state.push(Box::new(initial_state()));
    }

    NonUpdatingState {
        index,
        phantom_view_data: PhantomData
    }
}

impl <T: Any, ViewData: VViewData> NonUpdatingState<T, ViewData> {
    pub fn get<'a>(&'a self, c: &'a Box<VComponent<ViewData>>) -> &'a T {
        c.state
            .get(self.index).expect("unaligned hooks: state index out of bounds")
            .downcast_ref::<T>().expect("unaligned hooks: state type mismatch")
    }

    pub fn get_mut<'a>(&'a self, c: &'a mut Box<VComponent<ViewData>>) -> &'a mut T {
        self._get_mut(&mut c.state)
    }

    pub fn _get_mut<'a>(&'a self, c_state: &'a mut Vec<Box<dyn Any>>) -> &'a mut T {
        c_state
            .get_mut(self.index).expect("unaligned hooks: state index out of bounds")
            .downcast_mut::<T>().expect("unaligned hooks: state type mismatch")
    }
}

impl <T: Any, ViewData: VViewData> Clone for NonUpdatingState<T, ViewData> {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            phantom_view_data: self.phantom_view_data
        }
    }
}

impl <T: Any, ViewData: VViewData> Copy for NonUpdatingState<T, ViewData> {}