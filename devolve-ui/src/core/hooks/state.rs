//! Persistent state which a component retains from when it's created to when its destroyed
//! (a component is destroyed when its parent re-renders and the component is no longer in the parent's render.
//! Afterwards if it appears again it will be a new component with a reset state)
//!
//! You access a reference to the state with `State::get`, and a mutable reference with `State::get_mut`.
//! The latter will trigger a re-render when dropped, so only use when you intend to actually modify the state
//! or else you will get into an infinite loop.
//!
//! This type implements `Copy` so it can by pass between effect closures.
//! The references can't be passed, but this is actually ideal as they are stale.
//! The underlying type is actually just an index into the component, which stores the real state,
//! so it's cheap to pass around and will not de-sync with the component like JS React.
//! Be aware that if the underlying type is `Copy`, then you can *can* pass `State::get` and `State::get_mut`
//! values between closures, but they will be stale.

use std::any::Any;
use std::ops::{Deref, DerefMut};
use crate::core::component::component::VComponentHead;
use crate::core::component::context::{VComponentContext, VContext};
use crate::core::component::update_details::{UpdateBacktrace, UpdateDetails};
use crate::core::view::view::VViewData;
use crate::core::hooks::state_internal::{NonUpdatingState, use_non_updating_state};

#[derive(Debug)]
pub struct State<T: Any, ViewData: VViewData>(NonUpdatingState<T, ViewData>);

/// Smart pointer which allows access to the state, and calls `update` when it gets dropped.
pub struct StateDeref<'a, T: Any, ViewData: VViewData> {
    // See comment in StateDeref::drop
    component: *mut VComponentHead<ViewData>,
    update_details: UpdateDetails,
    value: &'a mut T
}

pub fn use_state<'a, T: Any, ViewData: VViewData + 'a, F: FnOnce() -> T>(
    c: &mut impl VComponentContext<'a, ViewData=ViewData>,
    initial_state: F
) -> State<T, ViewData> {
    State(use_non_updating_state(c, initial_state))
}

impl <T: Any, ViewData: VViewData> State<T, ViewData> {
    pub fn get<'a: 'b, 'b>(&self, c: &'b mut impl VContext<'a, ViewData=ViewData>) -> &'b T where ViewData: 'b {
        self.0.get(c)
    }

    pub fn get_mut<'a: 'b, 'b>(&self, c: &'b mut impl VContext<'a, ViewData=ViewData>) -> StateDeref<'b, T, ViewData> where ViewData: 'b {
        let update_details = UpdateDetails::SetState {
            index: self.0.index,
            backtrace: UpdateBacktrace::here()
        };
        // See comment in StateDeref::drop
        let component = c.component() as *mut _;
        StateDeref {
            component,
            update_details,
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
        let component = unsafe { self.component.as_mut().unwrap() };
        // Kind of a useless clone but we can't move
        component.update(self.update_details.clone());
    }
}

impl <T: Any, ViewData: VViewData> Clone for State<T, ViewData> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl <T: Any, ViewData: VViewData> Copy for State<T, ViewData> {}