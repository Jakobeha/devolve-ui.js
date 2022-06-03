/* use std::any::Any;
use crate::core::component::component::VComponent;
use crate::core::data::obs_ref::ObsRefableRoot;
use crate::core::hooks::state_internal::{NonUpdatingState, use_non_updating_state};

#[cfg(feature = "obs-ref")]
pub fn use_state_obs_ref<T: Any + ObsRefableRoot, ViewData: VViewData, F: FnOnce() -> T>(
    c: &mut Box<VComponent<ViewData>>,
    initial_state: F
) -> State<T::ObsRefImpl, ViewData> {
    let state = use_non_updating_state(c, || initial_state().into_obs_ref());
    state.get(c).after_mutate();
    state
} */
