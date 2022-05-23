use std::any::Any;
use std::marker::PhantomData;
use crate::core::component::component::VComponent;
use crate::core::view::view::VViewData;

pub struct NonUpdatingState<T: Any, ViewData: VViewData> {
    pub index: usize,
    pub phantom_view_data: PhantomData<ViewData>
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
    pub fn get(&self, c: &mut Box<VComponent<ViewData>>) -> &T {
        c.state
            .get(self.index).expect("unaligned hooks: state index out of bounds")
            .downcast_ref::<T>().expect("unaligned hooks: state type mismatch")
    }

    pub fn get_mut(&self, c: &mut Box<VComponent<ViewData>>) -> &mut T {
        c.state
            .get_mut(self.index).expect("unaligned hooks: state index out of bounds")
            .downcast_mut::<T>().expect("unaligned hooks: state type mismatch")
    }
}