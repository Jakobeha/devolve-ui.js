//! Single-threaded observable tree values: these aren't `Sync`.
//!
//! See `obs_ref` for more info.

/// Implementations for std primitives
pub mod leaf;
/// Implementations for vectors
pub mod vec;
/// Implementation for Zero-Sized types
pub mod zst;

use std::rc::{Rc, Weak};
use std::cell::RefCell;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};
use smallvec::SmallVec;

pub type Observer<Root, S> = Box<dyn Fn(&Root, &[<S as SubCtx>::Key], &str)>;

// region traits
/// Holds a mutable reference. Whenever you access it mutably,
/// it will trigger observers. You can access an observable reference to children of `ObsRefable`
pub trait ObsRef<Root, T, S: SubCtx> {
    /// Returns an immutable reference to the underlying value
    fn i(&self, s: S::Input<'_>) -> &T;

    /// Returns a mutable reference to the underlying value.
    /// When the reference is dropped, observers will be called
    fn m(&mut self, s: S::Input<'_>) -> ObsDeref<Root, T, S>;

    /// Add observer which will be called when `m` is called and then the reference is dropped.
    fn after_mutate(&self, observer: Observer<Root, S>);

    fn base(&self) -> &Rc<ObsRefRootBase<Root, S>>;
}

pub trait ObsRefableRoot<S: SubCtx>: Sized {
    type ObsRefImpl : ObsRef<Self, Self, S>;

    fn into_obs_ref(self) -> Self::ObsRefImpl;
}

pub trait ObsRefableChild<Root, S: SubCtx>: Sized {
    type ObsRefImpl : ObsRef<Root, Self, S>;

    unsafe fn _as_obs_ref_child(
        this: *mut Self,
        ancestors_pending: &[Weak<ObsRefPending<S>>],
        parent_pending: &Rc<ObsRefPending<S>>,
        path: String,
        root: Rc<ObsRefRootBase<Root, S>>
    ) -> Self::ObsRefImpl;

    unsafe fn as_obs_ref_child(
        &self,
        ancestors_pending: &[Weak<ObsRefPending<S>>],
        parent_pending: &Rc<ObsRefPending<S>>,
        path_head: &str,
        extension: &str,
        root: Rc<ObsRefRootBase<Root, S>>
    ) -> Self::ObsRefImpl {
        let path = if extension.starts_with('[') {
            format!("{}{}", path_head, extension)
        } else {
            format!("{}.{}", path_head, extension)
        };
        Self::_as_obs_ref_child(self as *const Self as *mut Self, ancestors_pending, parent_pending, path, root)
    }
}

/// Whenever you get a reference, you must pass a subscriber context.
/// Then when the reference is mutated, the propagator will receive all of the subscription keys
/// and can retrieve the contexts and forward updates to them.
pub trait SubCtx: 'static {
    type Input<'a>;
    type Key: Clone;

    fn convert_into_subscription_key(input: Self::Input<'_>) -> Self::Key;
}
// endregion

// region structs
pub struct ObsDeref<'a, Root, T, S: SubCtx> {
    // Has to be a pointer because we also store a reference to root.
    value: *mut T,
    parents_pending: &'a Vec<Weak<ObsRefPending<S>>>,
    path: &'a str,
    root: Option<&'a Rc<ObsRefRootBase<Root, S>>>,
}

// TODO: Do we need to pin Rc<ObsRefRootBase<T>>?
//   The children child_values reference root_value,
//   but they should't outlive root_value and shouldn't exist
//   if root_value has mutable access (since they also have mutable access)
pub struct ObsRefRootBase<T, S: SubCtx> {
    root_value: T,
    // SmallVec is ideal here because we usually don't have many observers
    observers: RefCell<SmallVec<[Observer<T, S>; 3]>>,
    pending: Rc<ObsRefPending<S>>
}

pub struct ObsRefChildBase<Root, T, S: SubCtx> {
    // self.root.value contains the real value
    // We don't store a lifetime because it's dependent on root, which is reference-counted.
    // However, we guarantee it won't be dangling.
    child_value: *mut T,
    pending: Rc<ObsRefPending<S>>,
    // The child clones its pending instances and sends them to parents. This ensures that
    // when the parent gets modified, the child is still observed.
    //
    // The child also runs parent's direct pending values. This ensures that
    // when the child gets modified, the parent is still observed.
    //
    // The root can't be dropped but intermediate parents can.
    // In that case, their Weak reference will be empty, but we can just ignore them
    // since you cannot modify them specifically.
    parents_pending: Vec<Weak<ObsRefPending<S>>>,
    path: String,
    root: Rc<ObsRefRootBase<Root, S>>,
}

pub struct ObsRefPending<S: SubCtx> {
    pub(super) direct: RefCell<SmallVec<[S::Key; 3]>>,
    // Child's pending values are also stored in each parent. This ensures that when the parent
    // gets modified, the child is still observed.
    pub(super) from_children: RefCell<SmallVec<[S::Key; 3]>>,
}
// endregion

// region struct impls
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
        let pending: Vec<S::Key> = parents_pending
            .iter()
            .filter_map(Weak::upgrade)
            .flat_map(|pending| pending.direct.borrow_mut().drain(..).collect::<Vec<_>>())
            .chain(self.pending.from_children.borrow_mut().drain(..))
            .collect();

        for observer in self.observers.borrow().iter() {
            observer(&self.root_value, &pending, path)
        }
    }

    pub fn pending(&self) -> &Rc<ObsRefPending<S>> {
        &self.pending
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

    pub fn parents_pending(&self) -> &[Weak<ObsRefPending<S>>] {
        &self.parents_pending
    }

    pub fn pending(&self) -> &Rc<ObsRefPending<S>> {
        &self.pending
    }

    fn push_pending(&self, s: S::Input<'_>) {
        let subscription = S::convert_into_subscription_key(s);
        for parent in self.parents_pending.iter() {
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


impl <'a, Root: 'a, T, S: SubCtx> ObsDeref<'a, Root, T, S> {
    const PARENTS_PENDING: Vec<Weak<ObsRefPending<S>>> = Vec::new();
    const PARENTS_PENDING_REF: &'static Vec<Weak<ObsRefPending<S>>> = &Self::PARENTS_PENDING;

    pub(super) fn zst(instance: &T) -> Self {
        ObsDeref {
            value: instance as *const T as *mut T,
            parents_pending: Self::PARENTS_PENDING_REF,
            path: "<zst deref>",
            root: None
        }
    }
}
// endregion

// region trait impls
impl <T, S: SubCtx> ObsRef<T, T, S> for Rc<ObsRefRootBase<T, S>> {
    fn i(&self, s: S::Input<'_>) -> &T {
        self.push_pending(s);

        &self.root_value
    }

    fn m(&mut self, s: S::Input<'_>) -> ObsDeref<'_, T, T, S> {
        self.push_pending(s);

        ObsDeref {
            value: &self.root_value as *const T as *mut T,
            parents_pending: ObsDeref::<'_, T, T, S>::PARENTS_PENDING_REF,
            path: "",
            root: Some(self),
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
            root: Some(&self.root),
        }
    }

    fn after_mutate(&self, observer: Observer<Root, S>) {
        self.root.observers.borrow_mut().push(observer);
    }

    #[allow(clippy::needless_lifetimes)]
    fn base<'a>(&'a self) -> &'a Rc<ObsRefRootBase<Root, S>> {
        &self.root
    }
}

impl <'a, Root, T, S: SubCtx> Deref for ObsDeref<'a, Root, T, S> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.value }
    }
}

impl <'a, Root, T, S: SubCtx> DerefMut for ObsDeref<'a, Root, T, S> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.value }
    }
}

impl <'a, Root, T, S: SubCtx> Drop for ObsDeref<'a, Root, T, S> {
    fn drop(&mut self) {
        if let Some(root) = self.root {
            root.send_update(self.parents_pending, self.path);
        }
    }
}

impl SubCtx for () {
    type Input<'a> = ();
    type Key = ();

    fn convert_into_subscription_key((): Self::Input<'_>) -> Self::Key {
        ()
    }
}
// endregion

// region boilerplate trait impls
impl <T: Debug, S: SubCtx> Debug for ObsRefRootBase<T, S> where S::Key: Debug {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ObsRefRootBase")
            .field("root_value", &self.root_value)
            .field("#observers", &self.observers.borrow().len())
            .field("pending", &self.pending)
            .finish()
    }
}

impl <Root: Debug, T: Debug, S: SubCtx> Debug for ObsRefChildBase<Root, T, S> where S::Key: Debug {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ObsRefChildBase")
            .field("child_value", &self.child_value)
            .field("parents_pending", &self.parents_pending)
            .field("path", &self.path)
            .field("pending", &self.pending)
            .field("root", &self.root)
            .finish()
    }
}

impl <'a, Root: Debug, T: Debug, S: SubCtx> Debug for ObsDeref<'a, Root, T, S> where S::Key: Debug {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ObsDeref")
            .field("value", unsafe { &*self.value })
            .field("parents_pending", &self.parents_pending)
            .field("path", &self.path)
            .field("root", &self.root)
            .finish()
    }
}

impl <S: SubCtx> Debug for ObsRefPending<S> where S::Key: Debug {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ObsRefPending")
            .field("direct", &self.direct)
            .field("from_children", &self.from_children)
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