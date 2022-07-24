//! Single-threaded observable tree values: these aren't `Sync`.
//!
//! See `obs_ref` for more info.

/// Implementations for std primitives
pub mod leaf;
/// Implementations for vectors
pub mod vec;

use std::rc::{Rc, Weak};
use std::cell::RefCell;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};
use smallvec::SmallVec;
use crate::misc::is_thread_safe::TSMutex;

// region types

pub union Observer<Root, S: SubCtx, const IS_THREAD_SAFE: bool> {
    yes: Box<dyn Fn(&Root, &[S::Key], &str) + Send>,
    no: Box<dyn Fn(&Root, &[S::Key], &str)>
}

/// Holds a mutable reference. Whenever you access it mutably,
/// it will trigger observers. You can access an observable reference to children of `ObsRefable`
pub trait ObsRef<Root, T, S: SubCtx, const IS_THREAD_SAFE: bool> {
    /// Returns an immutable reference to the underlying value
    fn i(&self, s: S::Input<'_>) -> &T;

    /// Returns a mutable reference to the underlying value.
    /// When the reference is dropped, observers will be called
    fn m(&mut self, s: S::Input<'_>) -> ObsDeref<Root, T, S, IS_THREAD_SAFE>;

    /// Add observer which will be called when `m` is called and then the reference is dropped.
    fn after_mutate(&self, observer: Observer<Root, S, IS_THREAD_SAFE>);

    fn base(&self) -> &TSRc<ObsRefRootBase<Root, S, IS_THREAD_SAFE>, IS_THREAD_SAFE>;
}

/// Whenever you get a reference, you must pass a subscriber context.
/// Then when the reference is mutated, the propagator will receive all of the subscription keys
/// and can retrieve the contexts and forward updates to them.
pub trait SubCtx {
    type Input<'a>;
    type Key: Clone;

    fn convert_into_subscription_key(input: Self::Input<'_>) -> Self::Key;
}

#[derive(Debug)]
pub struct ObsDeref<'a, Root, T, S: SubCtx, const IS_THREAD_SAFE: bool> {
    // Has to be a pointer because we also store a reference to root.
    value: *mut T,
    parents_pending: &'a Vec<TSWeak<ObsRefPending<S, IS_THREAD_SAFE>, IS_THREAD_SAFE>>,
    path: &'a str,
    root: TSRc<ObsRefRootBase<Root, S, IS_THREAD_SAFE>, IS_THREAD_SAFE>,
}

// TODO: Do we need to pin Rc<ObsRefRootBase<T>>?
//   The children child_values reference root_value,
//   but they should't outlive root_value and shouldn't exist
//   if root_value has mutable access (since they also have mutable access)
pub struct ObsRefRootBase<T, S: SubCtx, const IS_THREAD_SAFE: bool> {
    root_value: T,
    // SmallVec is ideal here because we usually don't have many observers
    observers: TSRwLock<SmallVec<[Observer<T, S, IS_THREAD_SAFE>; 3]>, IS_THREAD_SAFE>,
    pending: TSRc<ObsRefPending<S, IS_THREAD_SAFE>, IS_THREAD_SAFE>
}

#[derive(Debug)]
pub struct ObsRefChildBase<Root, T, S: SubCtx, const IS_THREAD_SAFE: bool> {
    // self.root.value contains the real value
    // We don't store a lifetime because it's dependent on root, which is reference-counted.
    // However, we guarantee it won't be dangling.
    child_value: *mut T,
    pending: TSRc<ObsRefPending<S>, IS_THREAD_SAFE>,
    // The child clones its pending instances and sends them to parents. This ensures that
    // when the parent gets modified, the child is still observed.
    //
    // The child also runs parent's direct pending values. This ensures that
    // when the child gets modified, the parent is still observed.
    //
    // The root can't be dropped but intermediate parents can.
    // In that case, their Weak reference will be empty, but we can just ignore them
    // since you cannot modify them specifically.
    parents_pending: Vec<TSWeak<ObsRefPending<S, IS_THREAD_SAFE>, IS_THREAD_SAFE>>,
    path: String,
    root: TSRc<ObsRefRootBase<Root, S, IS_THREAD_SAFE>, IS_THREAD_SAFE>,
}

#[derive(Debug)]
struct ObsRefPending<S: SubCtx, const IS_THREAD_SAFE: bool> {
    pub direct: TSMutex<SmallVec<[S::Key; 3]>, IS_THREAD_SAFE>,
    // Child's pending values are also stored in each parent. This ensures that when the parent
    // gets modified, the child is still observed.
    pub from_children: TSMutex<SmallVec<[S::Key; 3]>, IS_THREAD_SAFE>,
}

// endregion
// region traits

pub trait ObsRefableRoot<S: SubCtx>: Sized {
    type ObsRefImpl : ObsRef<Self, Self, S>;

    fn into_obs_ref(self: Self) -> Self::ObsRefImpl;
}

pub trait ObsRefableChild<Root, S: SubCtx>: Sized {
    type ObsRefImpl : ObsRef<Root, Self, S>;

    unsafe fn _as_obs_ref_child(this: *mut Self, path: String, root: Weak<ObsRefRootBase<Root, S>>) -> Self::ObsRefImpl;

    unsafe fn as_obs_ref_child(&mut self, path_head: &str, extension: &str, root: Weak<ObsRefRootBase<Root, S>>) -> Self::ObsRefImpl {
        let path = if extension.starts_with('[') {
            format!("{}{}", path_head, extension)
        } else {
            format!("{}.{}", path_head, extension)
        };
        Self::_as_obs_ref_child(self as *mut Self, path, root)
    }
}

// endregion
// region main impls

impl <T, S: SubCtx> ObsRefRootBase<T, S> {
    pub fn new(root_value: T) -> Rc<Self> {
        Rc::new(Self {
            root_value,
            observers: RefCell::new(SmallVec::new()),
            pending: Rc::new(ObsRefPending::new())
        })
    }

    pub fn root_value(&self) -> &T {
        &self.root_value
    }

    fn send_update(&self, parents_pending: &Vec<Weak<ObsRefPending<S>>>, path: &str) {
        let pending: Vec<S> = parents_pending
            .iter()
            .filter_map(Weak::upgrade)
            .map(|pending| pending.direct.borrow_mut().drain(..))
            .chain(self.pending.from_children.borrow_mut().drain(..))
            .collect();

        for observer in root.observers.borrow().iter() {
            observer(&self.root_value, &pending, path)
        }
    }

    fn push_pending(&self, s: S::Input<'_>) {
        let subscription = S::convert_into_subscription_key(s);
        self.pending.direct.borrow_mut().push(subscription);
    }
}

impl <Root, T, S: SubCtx> ObsRefChildBase<Root, T, S> {
    pub fn new(
        child_value: *mut T,
        ancestors_pending: &[Weak<ObsRefPending<S>>],
        parent_pending: &Rc<ObsRefPending<S>>,
        path: String,
        root: Rc<ObsRefRootBase<Root, S>>
    ) -> Self {
        let mut parents_pending = Vec::with_capacity(ancestors_pending.len() + 1);
        for ancestor_pending in ancestors_pending.iter().cloned() {
            parents_pending.push(ancestor_pending);
        }
        parents_pending.push(Rc::downgrade(parent_pending));

        Self {
            child_value,
            pending: Rc::new(ObsRefPending::new()),
            parents_pending,
            path,
            root
        }
    }

    pub fn child_value(&self) -> &T {
        unsafe { &*self.child_value }
    }

    pub fn path(&self) -> &str {
        self.path.as_str()
    }

    fn push_pending(&self, s: S::Input<'_>) {
        let subscription = S::convert_into_subscription_key(s);
        for parent in self.parents_pending {
            if let Some(parent) = parent.upgrade() {
                parent.from_children.borrow_mut().push(subscription.clone());
            }
        }
        self.pending.direct.borrow_mut().push(subscription);
    }
}

impl <S: SubCtx> ObsRefPending<S> {
    pub fn new() -> Self {
        ObsRefPending {
            direct: RefCell::new(SmallVec::new()),
            from_children: RefCell::new(SmallVec::new())
        }
    }
}

impl <T, S: SubCtx> ObsRef<T, T, S> for Rc<ObsRefRootBase<T, S>> {
    fn i(&self, s: S::Input<'_>) -> &T {
        self.push_pending(s);

        &self.root_value
    }

    fn m(&mut self, s: S::Input<'_>) -> ObsDeref<T, T, S> {
        self.push_pending(s);

        let value = &mut self.root_value as *mut T;
        ObsDeref {
            value,
            parents_pending: &Vec::new(),
            path: "",
            root: self.clone(),
        }
    }

    fn after_mutate(&self, observer: Observer<T, S>) {
        self.deref().observers.borrow_mut().push(observer)
    }

    fn base(&self) -> &Rc<ObsRefRootBase<T, S>> {
        self
    }
}

impl <Root, T, S: SubCtx> ObsRef<Root, T, S> for ObsRefChildBase<Root, T, S> {
    fn i(&self, s: S::Input<'_>) -> &T {
        self.push_pending(s);

        unsafe { &*self.child_value }
    }

    fn m(&mut self, s: S::Input<'_>) -> ObsDeref<Root, T, S> {
        self.push_pending(s);

        ObsDeref {
            value: unsafe { &mut *self.child_value },
            parents_pending: &self.parents_pending,
            path: &self.path,
            root: self.root.clone(),
        }
    }

    fn after_mutate(&self, observer: Observer<Root, S>) {
        let root = self.root.upgrade().expect("ObsRefableRoot weak ref is null");
        root.observers.borrow_mut().push(observer);
    }

    #[allow(clippy::needless_lifetimes)]
    fn base<'a>(&'a self) -> &'a Rc<ObsRefRootBase<Root, S>> {
        &self.root
    }
}

impl <'a, Root, T, S: SubCtx> Deref for ObsDeref<'a, Root, T, S> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

impl <'a, Root, T, S: SubCtx> DerefMut for ObsDeref<'a, Root, T, S> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl <'a, Root, T, S: SubCtx> Drop for ObsDeref<'a, Root, T, S> {
    fn drop(&mut self) {
        let root: &ObsRefRootBase<Root, S> = unsafe {
            self.root.as_ref().expect("ObsDeref root pointer is null")
        };
        root.send_update(self.parents_pending, self.path);
    }
}

// endregion
// region boilerplate impls

impl <T: Debug, S: SubCtx + Debug> Debug for ObsRefRootBase<T, S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ObsRefRootBase")
            .field("root_value", &self.root_value)
            .field("#observers", &self.observers.borrow().len())
            .field("pending", &self.pending)
            .finish()
    }
}

impl <T: PartialEq, S: SubCtx> PartialEq for ObsRefRootBase<T, S> {
    fn eq(&self, other: &Self) -> bool {
        self.root_value() == other.root_value()
    }
}

impl <T: PartialOrd, S: SubCtx> PartialOrd for ObsRefRootBase<T, S> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.root_value().partial_cmp(&other.root_value())
    }
}

impl <T: Hash, S: SubCtx> Hash for ObsRefRootBase<T, S> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.root_value().hash(state)
    }
}

impl <Root, T: PartialEq, S: SubCtx> PartialEq for ObsRefChildBase<Root, T, S> {
    fn eq(&self, other: &Self) -> bool {
        self.child_value() == other.child_value()
    }
}

impl <Root, T: PartialOrd, S: SubCtx> PartialOrd for ObsRefChildBase<Root, T, S> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.child_value().partial_cmp(&other.child_value())
    }
}

impl <Root, T: Hash, S: SubCtx> Hash for ObsRefChildBase<Root, T, S> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.child_value().hash(state)
    }
}
// endregion