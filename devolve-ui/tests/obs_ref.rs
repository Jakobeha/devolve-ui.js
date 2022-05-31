use devolve_ui::core::data::obs_ref::ObsRefableRoot;
use devolve_ui_derive::ObsRefable;

#[derive(Clone, ObsRefable)]
struct State {
    number: f64,
    indices: Vec<usize>,
    #[obs_ref(ignore)]
    id_which_should_be_readonly: usize
}

#[test]
fn test_obs_ref() {
    let state = State {
        number: 1.0,
        indices: vec![1, 2, 3],
        id_which_should_be_readonly: 0
    };
    let obs_ref = state.to_obs_ref();
}