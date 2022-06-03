#[cfg(feature = "backtrace")]
use backtrace::Backtrace;
use std::any::Any;
use std::borrow::Cow;
use std::ops::{Deref, DerefMut};
use crate::core::component::component::VComponent;
use crate::core::view::view::VViewData;
use crate::core::hooks::state_internal::{NonUpdatingState, use_non_updating_state};

pub struct State<T: Any, ViewData: VViewData>(NonUpdatingState<T, ViewData>);

/// Smart pointer which allows access to the state, and calls `update` when it gets dropped.
pub struct StateDeref<'a, T : Any, ViewData: VViewData> {
    // See comment in StateDeref::drop
    c: *mut Box<VComponent<ViewData>>,
    update_message: String,
    value: &'a mut T
}

pub fn use_state<T: Any, ViewData: VViewData, F: FnOnce() -> T>(
    c: &mut Box<VComponent<ViewData>>,
    initial_state: F
) -> State<T, ViewData> {
    State(use_non_updating_state(c, initial_state))
}

impl <T: Any, ViewData: VViewData> State<T, ViewData> {
    pub fn get<'a>(&'a self, c: &'a Box<VComponent<ViewData>>) -> &'a T {
        self.0.get(c)
    }

    pub fn get_mut<'a>(&'a mut self, c: &'a mut Box<VComponent<ViewData>>) -> StateDeref<'a, T, ViewData> {
        #[cfg(feature = "backtrace")]
            let backtrace = Backtrace::new();
        #[cfg(not(feature = "backtrace"))]
            let backtrace = "<backtrace not used>";
        let update_message = format!("set:state{}\n{:?}", self.0.index, backtrace);
        StateDeref {
            // See comment in StateDeref::drop
            c: c as *mut Box<VComponent<ViewData>>,
            update_message,
            value: self.0.get_mut(c)
        }
    }
}

impl <'a, T: Any, ViewData: VViewData> Deref for StateDeref<'a, T, ViewData> {
    type Target = T;

    fn deref(&self) -> &T {
        self.value
    }
}

impl <'a, T: Any, ViewData: VViewData> DerefMut for StateDeref<'a, T, ViewData> {
    fn deref_mut(&mut self) -> &mut T {
        self.value
    }
}

impl <'a, T: Any, ViewData: VViewData> Drop for StateDeref<'a, T, ViewData> {
    fn drop(&mut self) {
        // We must "borrow c mutably twice", in c and self.0.get_mut(c).
        // However, this is sound because we never use both of these simultaneously.
        // value is used in deref and deref_mut. c is not used until drop.
        // deref and deref_mut will never be called in drop and vice versa.
        let c = unsafe { self.c.as_mut().unwrap() };
        c.update(Cow::Owned(self.update_message.clone()));
    }
}