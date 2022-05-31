use devolve_ui::core::data::obs_ref::{ObsRef, ObsRefableRoot};
use devolve_ui_derive::ObsRefable;

#[derive(Clone, ObsRefable)]
struct State {
    number: f64,
    indices: Vec<usize>,
    #[obs_ref(ignore)]
    #[allow(dead_code)]
    id_which_should_be_readonly: usize
}

#[test]
fn test_obs_ref() {
    let state = State {
        number: 1.0,
        indices: vec![1, 2, 3],
        id_which_should_be_readonly: 0
    };
    let mut obs_ref = state.to_obs_ref();

    assert_eq!(*obs_ref.number.i(), 1.0f64);
    *obs_ref.number.m() = 2.0;
    assert_eq!(*obs_ref.number.i(), 2.0f64);

    assert_eq!(*obs_ref.indices[1].i(), 2);
    *obs_ref.indices[1].m() += 5;
    assert_eq!(*obs_ref.indices[1].i(), 7);
    obs_ref.indices.m().remove(1);
    assert_eq!(obs_ref.indices.i().len(), 2);
    assert_eq!(*obs_ref.indices[1].i(), 3);
}