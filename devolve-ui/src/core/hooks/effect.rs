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
use crate::core::component::context::{VComponentContext1, VDestructorContext2, VEffectContext2, with_destructor_context};
use crate::core::view::view::VViewData;
use crate::core::hooks::state_internal::InternalHooks;

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

pub(super) fn _use_effect<
    'a,
    'a0: 'a,
    Props : Any,
    Destructor: FnOnce(VDestructorContext2<'_, '_, Props, ViewData>) + 'static,
    ViewData: VViewData + 'static
>(
    c: &mut VComponentContext1<'a, 'a0, Props, ViewData>,
    rerun: UseEffectRerun<NoDependencies>,
    effect: impl Fn(VEffectContext2<'_, '_, Props, ViewData>) -> Destructor + 'static
) {
    _use_effect_with_deps(c, rerun, effect);
}

pub(super) fn _use_effect_on_create<
    'a,
    'a0: 'a,
    Props : Any,
    Destructor: FnOnce(VDestructorContext2<'_, '_, Props, ViewData>) + 'static,
    ViewData: VViewData + 'static
>(
    c: &mut VComponentContext1<'a, 'a0, Props, ViewData>,
    effect: impl FnOnce(VEffectContext2<'_, '_, Props, ViewData>) -> Destructor + 'static
) {
    let effect = RefCell::new(Some(effect));
    _use_effect(c, UseEffectRerun::OnCreate, move |c| {
        let effect = effect.borrow_mut().take().expect("unexpected: use_effect_on_create's effect requested multiple times");
        effect(c)
    })
}

pub(super) fn _use_effect_with_deps<
    'a,
    'a0: 'a,
    Props: Any,
    Dependencies: CollectionOfPartialEqs + 'static,
    Destructor: FnOnce(VDestructorContext2<'_, '_, Props, ViewData>) + 'static,
    ViewData: VViewData + 'static
>(
    c: &mut VComponentContext1<'a, 'a0, Props, ViewData>,
    rerun: UseEffectRerun<Dependencies>,
    effect: impl Fn(VEffectContext2<'_, '_, Props, ViewData>) -> Destructor + 'static
) {
    match rerun {
        UseEffectRerun::OnCreate => {
            if c.component.is_being_created() {
                c.effects.effects.push(Box::new(move |(mut c, props)| {
                    let destructor = c.with(|c| effect((c, props)));
                    c.destructors().permanent_destructors.push(Box::new(move |c| destructor(c)));
                }))
            }
        },
        UseEffectRerun::OnUpdate => {
            c.effects.effects.push(Box::new(move |(mut c, props)| {
                let destructor = c.with(|c| effect((c, props)));
                c.destructors().update_destructors.push(Box::new(|c| destructor(c)));
            }));
        },
        UseEffectRerun::OnChange(mut dependencies) => {
            let is_created = c.component.is_being_created();
            // on mem::replace - we want to move dependencies, because we know that it will only be
            // moved when is_created is true, and when is_created is true we also don't access dependencies
            // afterwards. However, Rust won't allow that, and I'm pretty sure it would cause undefined
            // behavior. Se we use mem::replace with an empty vector, which is cheap and satisfies the borrow checker
            // (see https://stackoverflow.com/questions/48141703/is-there-a-way-to-force-rust-to-let-me-use-a-possibly-moved-value)
            let memo = c.use_non_updating_state(|| mem::replace(&mut dependencies, Dependencies::empty()));
            let destructor_index = c.use_non_updating_state::<i32>(|| -1);
            // This is not just to satisfy the borrow checker: we want to get the old dependencies
            // and then set the new ones, and mem::replace happens to be the perfect tool for this.
            let old_dependencies = mem::replace(memo.get_mut(c), dependencies);

            c.effects.effects.push(Box::new(move |(mut c, props)| {
                let do_effect = if is_created {
                    true
                } else {
                    let dependencies = memo.get(&mut c);
                    assert_eq!(old_dependencies.len(), dependencies.len(), "number of dependencies changed in between component update; you can't do that, only change the dependencies themselves, instead replace with nulls");
                    old_dependencies.iter().zip(dependencies.iter()).any(|(old_dep, new_dep)| old_dep != new_dep)
                };
                if do_effect {
                    let current_destructor_index = *destructor_index.get(&mut c);
                    if current_destructor_index != -1 {
                        c.with(|mut c| {
                            let cd = c.destructors();
                            // We can't screw up indices for other OnChange operations, so we replace with a no-op closure
                            let old_destructor = mem::replace(cd.permanent_destructors.get_mut(current_destructor_index as usize).unwrap(), Box::new(|_| ()));
                            with_destructor_context((&mut c, props), old_destructor);
                        });
                    }
                    let new_destructor = c.with(|c| effect((c, props)));
                    let (co, cd) = c.component_and_destructors();
                    *destructor_index._get_mut(&mut co.h.state) = cd.permanent_destructors.len() as i32;
                    cd.permanent_destructors.push(Box::new(move |c| new_destructor(c)));
                }
            }));
        },
        UseEffectRerun::OnPredicate(predicate) => {
            let memo = c.use_non_updating_state(|| false);
            let destructor_index = c.use_non_updating_state::<i32>(|| -1);
            let old_predicate = mem::replace(memo.get_mut(c), predicate);

            c.effects.effects.push(Box::new(move |(mut c, props)| {
                if predicate && !old_predicate {
                    // Run effect
                    let destructor = c.with(|c| effect((c, props)));
                    let (co, cd) = c.component_and_destructors();
                    *destructor_index._get_mut(&mut co.h.state) = cd.permanent_destructors.len() as i32;
                    cd.permanent_destructors.push(Box::new(move |c| destructor(c)));
                } else if !predicate && old_predicate {
                    // Run destructor
                    let current_destructor_index = *destructor_index.get(&mut c);
                    let cd = c.destructors();
                    let destructor = mem::replace(cd.permanent_destructors.get_mut(current_destructor_index as usize).unwrap(), Box::new(|_| ()));
                    with_destructor_context((&mut c, props), destructor);
                }
            }))
        },
        UseEffectRerun::OnChangeAndPredicate { mut dependencies, predicate } => {
            let memo = c.use_non_updating_state::<(Dependencies, bool)>(|| (mem::replace(&mut dependencies, Dependencies::empty()), false));
            let destructor_index = c.use_non_updating_state::<i32>(|| -1);
            let (old_dependencies, old_predicate) = mem::replace(memo.get_mut(c), (dependencies, false));

            c.effects.effects.push(Box::new(move |(mut c, props)| {
                if predicate && !old_predicate {
                    // Run effect
                    let destructor = c.with(|c| effect((c, props)));
                    let (co, cd) = c.component_and_destructors();
                    *destructor_index._get_mut(&mut co.h.state) = cd.permanent_destructors.len() as i32;
                    cd.permanent_destructors.push(Box::new(move |(c, props)| destructor((c, props))));
                } else if !predicate && old_predicate {
                    // Run destructor
                    let current_destructor_index = *destructor_index.get(&mut c);
                    let cd = c.destructors();
                    let destructor = mem::replace(cd.permanent_destructors.get_mut(current_destructor_index as usize).unwrap(), Box::new(|_| ()));
                    with_destructor_context((&mut c, props), destructor);
                } else if predicate && old_predicate {
                    let (dependencies, _) = memo.get(&mut c);
                    let do_effect = {
                        assert_eq!(old_dependencies.len(), dependencies.len(), "number of dependencies changed in between component update: you can't do that, only change the dependencies themselves, instead replace with nulls");
                        old_dependencies.iter().zip(dependencies.iter()).any(|(old_dep, new_dep)| old_dep != new_dep)
                    };
                    if do_effect {
                        // Run destructor and then predicate if dependencies change
                        let current_destructor_index = *destructor_index.get(&mut c);
                        if current_destructor_index != -1 {
                            let cd = c.destructors();
                            // We can't screw up indices for other OnChange operations, so we replace with a no-op closure
                            let old_destructor = mem::replace(cd.permanent_destructors.get_mut(current_destructor_index as usize).unwrap(), Box::new(|_| ()));
                            with_destructor_context((&mut c, props), old_destructor);
                        }
                        let new_destructor = c.with(|c| effect((c, props)));
                        let (co, cd) = c.component_and_destructors();
                        *destructor_index._get_mut(&mut co.h.state) = cd.permanent_destructors.len() as i32;
                        cd.permanent_destructors.push(Box::new(move |c| new_destructor(c)));
                    }
                }
            }))
        }
    }
}