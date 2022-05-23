#[cfg(feature = "backtrace")]
use backtrace::Backtrace;
use std::any::Any;
use std::borrow::Cow;
use crate::core::component::component::VComponent;
use crate::core::view::view::VViewData;
use crate::core::hooks::state_internal::{NonUpdatingState, use_non_updating_state};

pub struct State<T: Any, ViewData: VViewData>(NonUpdatingState<T, ViewData>);

pub fn use_state<T: Any, ViewData: VViewData>(c: &mut Box<VComponent<ViewData>>, initial_state: impl FnOnce() -> T) -> State<T, ViewData> {
    State(use_non_updating_state(c, initial_state))
}

impl <T: Any, ViewData: VViewData> State<T, ViewData> {
    pub fn get<'a>(&'a self, c: &'a Box<VComponent<ViewData>>) -> &'a T {
        self.0.get(c)
    }

    pub fn set(&self, c: &mut Box<VComponent<ViewData>>, new_value: T) {
        *self.0.get_mut(c) = new_value;
        #[cfg(feature = "backtrace")]
            let backtrace = Backtrace::new();
        #[cfg(not(feature = "backtrace"))]
            let backtrace = "<backtrace not used>";
        c.update(Cow::Owned(format!("set:state{}\n{:?}", self.0.index, backtrace)));
    }
}