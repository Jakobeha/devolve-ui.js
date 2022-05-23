use std::any::Any;
use std::mem;
use crate::core::component::component::VComponent;
use crate::core::data::data::Data;
use crate::core::view::view::VViewData;
use crate::core::hooks::state_internal::{NonUpdatingState, use_non_updating_state};

/// Determines when an effect closure is run. The destructor is run every time before the closure
/// is rerun, and before the component is destroyed.
pub enum UseEffectRerun {
    /// Called only once when the component is created.
    /// The destructor is called only when the component is destroyed
    OnCreate,
    /// Called when the component is created, then every time it updates,
    /// The destructor is called every time the component updates until it is destroyed
    OnUpdate,
    /// Called when the component is created, then
    /// called when any of the argument change (pass a vec with one element for one argument)
    /// The number of arguments must stay the same (pass null if you have arguments which come/go).
    /// The destructor is called before the next change, and when the component is destroyed
    OnChange(Vec<Box<dyn Data>>),
    /// Called the first time the predicate is true, which is not necessarily when the component is created
    /// Then called when the predicate is set to false, or the component is destroyed.
    /// Then called again if the predicate becomes true, and so on.
    /// However, if the predicate is true and then stays true in a subsequent update, it isn't called again.
    OnPredicate(bool),
    /// Combines `OnChange` and `OnPredicate`: that is, called the first time the predicate is true,
    /// which is not necessarily when the component is created, and then called subsequently if
    /// a) the predicate becomes false and then true again, or b) the predicate stays true but the
    /// dependencies change. The destructor is called when a) the predicate is set to false, b)
    /// before the next call when the dependencies change, and c) when the component is destroyed.
    OnChangeAndPredicate {
        dependencies: Vec<Box<dyn Data>>,
        predicate: bool
    }
}

/// Runs a closure according to `rerun`. The closure should contain an effect,
/// while the component's body should otherwise be a "pure" function based on its
/// props and state hooks like `use_state`.
pub fn use_effect<ViewData: VViewData, Destructor: FnOnce(&mut Box<VComponent<ViewData>>) -> ()>(c: &mut Box<VComponent<ViewData>>, rerun: UseEffectRerun, effect: impl Fn(&mut Box<VComponent<ViewData>>) -> Destructor) {
    match rerun {
        UseEffectRerun::OnCreate => {
            if c.is_being_created() {
                c.effects.push(Box::new(move |c| {
                    let destructor = effect(c);
                    c.permanent_destructors.push(Box::new(move |c| destructor(c)));
                }))
            }
        },
        UseEffectRerun::OnUpdate => {
            c.effects.push(Box::new(move |c| {
                let destructor = effect(c);
                c.update_destructors.push(Box::new(|c| destructor(c)));
            }));
        },
        UseEffectRerun::OnChange(mut dependencies) => {
            let is_created = c.is_being_created();
            // on mem::replace - we want to move dependencies, because we know that it will only be
            // moved when is_created is true, and when is_created is true we also don't access dependencies
            // afterwards. However, Rust won't allow that, and I'm pretty sure it would cause undefined
            // behavior. Se we use mem::replace with an empty vector, which is cheap and satisfies the borrow checker
            // (see https://stackoverflow.com/questions/48141703/is-there-a-way-to-force-rust-to-let-me-use-a-possibly-moved-value)
            let memo = use_non_updating_state(c, || mem::replace(&mut dependencies, vec![]));
            let destructor_index = use_non_updating_state::<i32, _>(c, || -1);
            // This is not just to satisfy the borrow checker: we want to get the old dependencies
            // and then set the new ones, and mem::replace happens to be the perfect tool for this.
            let old_dependencies = mem::replace(memo.get_mut(c), dependencies);

            c.effects.push(Box::new(move |c| {
                let do_effect = if is_created {
                    true
                } else {
                    assert_eq!(old_dependencies.len(), dependencies.len(), "number of dependencies changed in between component update; you can't do that, only change the dependencies themselves, instead replace with nulls");
                    old_dependencies.iter().zip(dependencies.iter()).any(|(old_dep, new_dep)| old_dep != new_dep)
                };
                if do_effect {
                    let mut destructor_index = destructor_index.get_mut(c);
                    if *destructor_index != -1 {
                        // We can't screw up indices for other OnChange operations, so we replace with a no-op closure
                        let old_destructor = mem::replace(c.permanent_destructors.get_mut(*destructor_index).unwrap(), Box::new(|_| ()));
                        old_destructor(c);
                    }
                    let new_destructor = effect(c);
                    *destructor_index = c.permanent_destructors.len() as i32;
                    c.permanent_destructors.push(Box::new(move |c| new_destructor(c)));
                }
            }));
        },
        UseEffectRerun::OnPredicate(predicate) => {
            let memo = use_non_updating_state(c, || false);
            let destructor_index = use_non_updating_state::<i32, _>(c, || -1);
            let old_predicate = mem::replace(memo.get_mut(c), predicate);

            c.effects.push(Box::new(move |c| {
                if predicate && !old_predicate {
                    // Run effect
                    let destructor = effect(c);
                    *destructor_index.get_mut(c) = c.permanent_destructors.len() as i32;
                    c.permanent_destructors.push(Box::new(move |c| destructor(c)));
                } else if !predicate && old_predicate {
                    // Run destructor
                    let destructor = mem::replace(c.permanent_destructors.get_mut(*destructor_index.get(c)).unwrap(), Box::new(|_| ()));
                    destructor(c);
                }
            }))
        },
        UseEffectRerun::OnChangeAndPredicate { mut dependencies, predicate } => {
            let memo = use_non_updating_state::<(Vec<Box<dyn Data>>, bool), _>(c, || (mem::replace(&mut dependencies, vec![]), false));
            let destructor_index = use_non_updating_state::<i32, _>(c, || -1);
            let (old_dependencies, old_predicate) = mem::replace(memo.get_mut(c), (dependencies, false));

            c.effects.push(Box::new(move |c| {
                if predicate && !old_predicate {
                    // Run effect
                    let destructor = effect(c);
                    *destructor_index.get_mut(c) = c.permanent_destructors.len() as i32;
                    c.permanent_destructors.push(Box::new(move |c| destructor(c)));
                } else if !predicate && old_predicate {
                    // Run destructor
                    let destructor = mem::replace(c.permanent_destructors.get_mut(*destructor_index.get(c)).unwrap(), Box::new(|_| ()));
                    destructor(c);
                } else if predicate && old_predicate {
                    let do_effect = {
                        assert_eq!(old_dependencies.len(), dependencies.len(), "number of dependencies changed in between component update: you can't do that, only change the dependencies themselves, instead replace with nulls");
                        old_dependencies.iter().zip(dependencies.iter()).any(|(old_dep, new_dep)| old_dep != new_dep)
                    };
                    if do_effect {
                        // Run destructor and then predicate if dependencies change
                        let mut destructor_index = destructor_index.get_mut(c);
                        if *destructor_index != -1 {
                            // We can't screw up indices for other OnChange operations, so we replace with a no-op closure
                            let old_destructor = mem::replace(c.permanent_destructors.get_mut(*destructor_index).unwrap(), Box::new(|_| ()));
                            old_destructor(c);
                        }
                        let new_destructor = effect(c);
                        *destructor_index = c.permanent_destructors.len() as i32;
                        c.permanent_destructors.push(Box::new(move |c| new_destructor(c)));
                    }
                }
            }))
        }
    }
}