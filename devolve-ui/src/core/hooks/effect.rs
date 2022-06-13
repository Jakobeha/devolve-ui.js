//! Run side-effects in components, and in particular, don't run them during every update.
//! The code inside the component's function body itself should be fast,
//! because it will be called every time the component updates.
//! According to good design, it should also be "pure" and side effects should be distinctly marked
//! by being in `use_effect` closures.

use std::any::Any;
use std::cell::RefCell;
use std::convert::Infallible;
use std::mem;
use std::slice::Iter;
use crate::core::component::context::{VComponentContextImpl, VContext, VDestructorContextImpl, VEffectContextImpl};
use crate::core::view::view::VViewData;
use crate::core::hooks::state_internal::use_non_updating_state;

pub trait CollectionOfPartialEqs {
    type Item: PartialEq;

    fn empty() -> Self;
    fn len(&self) -> usize;
    fn iter(&self) -> Iter<'_, Self::Item>;
}

impl <T: PartialEq> CollectionOfPartialEqs for Vec<T> {
    type Item = T;

    fn empty() -> Self {
        Vec::new()
    }

    fn len(&self) -> usize {
        Vec::len(self)
    }

    fn iter(&self) -> Iter<'_, Self::Item> {
        (self as &[T]).iter()
    }
}

impl CollectionOfPartialEqs for Infallible {
    type Item = Infallible;

    fn empty() -> Self {
        panic!("CollectionOfPartialEqs::empty() called on NoDependencies")
    }

    fn len(&self) -> usize {
        panic!("CollectionOfPartialEqs::len() called on NoDependencies")
    }

    fn iter(&self) -> Iter<'_, Self::Item> {
        panic!("CollectionOfPartialEqs::iter() called on NoDependencies")
    }
}

/// Determines when an effect closure is run. The destructor is run every time before the closure
/// is rerun, and before the component is destroyed.
pub enum UseEffectRerun<Dependencies : CollectionOfPartialEqs> {
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
    OnChange(Dependencies),
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
        dependencies: Dependencies,
        predicate: bool
    }
}

pub type NoDependencies = Infallible;

/// Runs a closure according to `rerun`. The closure should contain an effect,
/// while the component's body should otherwise be a "pure" function based on its
/// props and state hooks like `use_state`.
pub fn use_effect<
    Props : Any,
    Destructor: FnOnce(&mut VDestructorContextImpl<'_, Props, ViewData>) + 'static,
    ViewData: VViewData + 'static
>(
    c: &mut VComponentContextImpl<'_, Props, ViewData>,
    rerun: UseEffectRerun<NoDependencies>,
    effect: impl Fn(&mut VEffectContextImpl<'_, Props, ViewData>) -> Destructor + 'static
) {
    use_effect_with_deps(c, rerun, effect);
}

/// Runs a closure once on create. The closure should contain an effect,
/// while the component's body should otherwise be a "pure" function based on its
/// props and state hooks like `use_state`.
///
/// The behavior is exactly like `use_effect` and `use_effect_with_deps` when given `UseEffectRerun::OnCreate`.
/// However, this function allows you to pass an `FnOnce` to `effect` since we statically know it will only be called once.
pub fn use_effect_on_create<
    Props : Any,
    Destructor: FnOnce(&mut VDestructorContextImpl<'_, Props, ViewData>) + 'static,
    ViewData: VViewData + 'static
>(
    c: &mut VComponentContextImpl<'_, Props, ViewData>,
    effect: impl FnOnce(&mut VEffectContextImpl<'_, Props, ViewData>) -> Destructor + 'static
) {
    let effect = RefCell::new(Some(effect));
    use_effect(c, UseEffectRerun::OnCreate, move |c| {
        let effect = effect.borrow_mut().take().expect("unexpected: use_effect_on_create's effect requested multiple times");
        effect(c)
    })
}

/// Runs a closure according to `rerun`. The closure should contain an effect,
/// while the component's body should otherwise be a "pure" function based on its
/// props and state hooks like `use_state`.
///
/// This function is actually the exact same as `use_effect`, but exposes the dependencies as a type parameter.
/// Without the 2 versions, you would always have to specify dependencies on `use_effect` even if the enum variant didn't have them.
pub fn use_effect_with_deps<
    Props : Any,
    Dependencies: CollectionOfPartialEqs + 'static,
    Destructor: FnOnce(&mut VDestructorContextImpl<'_, Props, ViewData>) + 'static,
    ViewData: VViewData + 'static
>(
    c: &mut VComponentContextImpl<'_, Props, ViewData>,
    rerun: UseEffectRerun<Dependencies>,
    effect: impl Fn(&mut VEffectContextImpl<'_, Props, ViewData>) -> Destructor + 'static
) {
    let (co, ce) = c.component_and_effects();
    match rerun {
        UseEffectRerun::OnCreate => {
            if co.is_being_created() {
                ce.effects.push(Box::new(move |c| {
                    let destructor = effect(c);
                    c.permanent_destructors.push(Box::new(move |c| destructor(c)));
                }))
            }
        },
        UseEffectRerun::OnUpdate => {
            ce.effects.push(Box::new(move |c| {
                let destructor = effect(c);
                c.destructors.update_destructors.push(Box::new(|c| destructor(c)));
            }));
        },
        UseEffectRerun::OnChange(mut dependencies) => {
            let is_created = co.is_being_created();
            // on mem::replace - we want to move dependencies, because we know that it will only be
            // moved when is_created is true, and when is_created is true we also don't access dependencies
            // afterwards. However, Rust won't allow that, and I'm pretty sure it would cause undefined
            // behavior. Se we use mem::replace with an empty vector, which is cheap and satisfies the borrow checker
            // (see https://stackoverflow.com/questions/48141703/is-there-a-way-to-force-rust-to-let-me-use-a-possibly-moved-value)
            let memo = use_non_updating_state(c, || mem::replace(&mut dependencies, Dependencies::empty()));
            let destructor_index = use_non_updating_state::<i32, _>(c, || -1);
            // This is not just to satisfy the borrow checker: we want to get the old dependencies
            // and then set the new ones, and mem::replace happens to be the perfect tool for this.
            let old_dependencies = mem::replace(memo.get_mut(c), dependencies);

            ce.effects.push(Box::new(move |c| {
                let (co, cd) = c.component_and_destructors();
                let do_effect = if is_created {
                    true
                } else {
                    let dependencies = memo.get(c);
                    assert_eq!(old_dependencies.len(), dependencies.len(), "number of dependencies changed in between component update; you can't do that, only change the dependencies themselves, instead replace with nulls");
                    old_dependencies.iter().zip(dependencies.iter()).any(|(old_dep, new_dep)| old_dep != new_dep)
                };
                if do_effect {
                    let current_destructor_index = *destructor_index.get(c);
                    if current_destructor_index != -1 {
                        // We can't screw up indices for other OnChange operations, so we replace with a no-op closure
                        let old_destructor = mem::replace(cd.permanent_destructors.get_mut(current_destructor_index as usize).unwrap(), Box::new(|_| ()));
                        old_destructor(c.into_destructor_context());
                    }
                    let new_destructor = effect(c);
                    *destructor_index._get_mut(&mut co.h.state) = cd.permanent_destructors.len() as i32;
                    cd.permanent_destructors.push(Box::new(move |c| new_destructor(c)));
                }
            }));
        },
        UseEffectRerun::OnPredicate(predicate) => {
            let memo = use_non_updating_state(c, || false);
            let destructor_index = use_non_updating_state::<i32, _>(c, || -1);
            let old_predicate = mem::replace(memo.get_mut(c), predicate);

            ce.effects.push(Box::new(move |c| {
                let (co, cd) = c.component_and_destructors();
                if predicate && !old_predicate {
                    // Run effect
                    let destructor = effect(c);
                    *destructor_index._get_mut(&mut co.h.state) = cd.permanent_destructors.len() as i32;
                    cd.permanent_destructors.push(Box::new(move |c| destructor(c)));
                } else if !predicate && old_predicate {
                    // Run destructor
                    let current_destructor_index = *destructor_index.get(c);
                    let destructor = mem::replace(cd.permanent_destructors.get_mut(current_destructor_index as usize).unwrap(), Box::new(|_| ()));
                    destructor(c.into_destructor_context());
                }
            }))
        },
        UseEffectRerun::OnChangeAndPredicate { mut dependencies, predicate } => {
            let memo = use_non_updating_state::<(Dependencies, bool), _>(c, || (mem::replace(&mut dependencies, Dependencies::empty()), false));
            let destructor_index = use_non_updating_state::<i32, _>(c, || -1);
            let (old_dependencies, old_predicate) = mem::replace(memo.get_mut(c), (dependencies, false));

            ce.effects.push(Box::new(move |c| {
                let (co, cd) = c.component_and_destructors();
                if predicate && !old_predicate {
                    // Run effect
                    let destructor = effect(c);
                    *destructor_index._get_mut(&mut co.h.state) = cd.permanent_destructors.len() as i32;
                    cd.permanent_destructors.push(Box::new(move |c| destructor(c)));
                } else if !predicate && old_predicate {
                    // Run destructor
                    let current_destructor_index = *destructor_index.get(c);
                    let destructor = mem::replace(cd.permanent_destructors.get_mut(current_destructor_index as usize).unwrap(), Box::new(|_| ()));
                    destructor(c.into_destructor_context());
                } else if predicate && old_predicate {
                    let (dependencies, _) = memo.get(c);
                    let do_effect = {
                        assert_eq!(old_dependencies.len(), dependencies.len(), "number of dependencies changed in between component update: you can't do that, only change the dependencies themselves, instead replace with nulls");
                        old_dependencies.iter().zip(dependencies.iter()).any(|(old_dep, new_dep)| old_dep != new_dep)
                    };
                    if do_effect {
                        // Run destructor and then predicate if dependencies change
                        let current_destructor_index = *destructor_index.get(c);
                        if current_destructor_index != -1 {
                            // We can't screw up indices for other OnChange operations, so we replace with a no-op closure
                            let old_destructor = mem::replace(cd.permanent_destructors.get_mut(current_destructor_index as usize).unwrap(), Box::new(|_| ()));
                            old_destructor(c.into_destructor_context());
                        }
                        let new_destructor = effect(c);
                        *destructor_index._get_mut(&mut co.h.state) = cd.permanent_destructors.len() as i32;
                        cd.permanent_destructors.push(Box::new(move |c| new_destructor(c)));
                    }
                }
            }))
        }
    }
}