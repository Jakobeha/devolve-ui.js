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

// region types

pub type Observer<Root, Assoc> = Box<dyn Fn(&Root, &[Assoc], &str) -> ()>;

/// Holds a mutable reference. Whenever you access it mutably,
/// it will trigger observers. You can access an observable reference to children of `ObsRefable`
pub trait ObsRef<Root, T, Assoc: ObsRefAssoc> {
    /// Returns an immutable reference to the underlying value
    fn i(&self, assoc: Assoc::Input) -> &T;

    /// Returns a mutable reference to the underlying value.
    /// When the reference is dropped, observers will be called
    fn m(&mut self, assoc: Assoc::Input) -> ObsDeref<Root, T, Assoc>;

    /// Add observer which will be called when `m` is called and then the reference is dropped.
    fn after_mutate(&self, observer: Observer<Root, Assoc>);

    fn base(&self) -> &Rc<ObsRefRootBase<Root, Assoc>>;
}

/// Associated values which must be passed whenever you get a reference.
/// These are particularly useful to track changes: the associated value stores the parent
/// which wanted the reference, so when the value gets modified the reference will change.
///
/// TODO: Explain better
pub trait ObsRefAssoc: Clone {
    type Input;

    fn from_obs_ref_assoc_input(input: Self::Input) -> Self;
}

#[derive(Debug)]
pub struct ObsDeref<'a, Root, T, Assoc: ObsRefAssoc> {
    // Has to be a pointer because we also store a reference to root.
    value: *mut T,
    parents_pending: &'a Vec<Weak<ObsRefPending<Assoc>>>,
    path: &'a str,
    root: Rc<ObsRefRootBase<Root, Assoc>>,
}

// TODO: Do we need to pin Rc<ObsRefRootBase<T>>?
//   The children child_values reference root_value,
//   but they should't outlive root_value and shouldn't exist
//   if root_value has mutable access (since they also have mutable access)
pub struct ObsRefRootBase<T, Assoc: ObsRefAssoc> {
    root_value: T,
    // SmallVec is ideal here because we usually don't have many observers
    observers: RefCell<SmallVec<[Observer<T, Assoc>; 3]>>,
    pending: Rc<ObsRefPending<Assoc>>
}

#[derive(Debug)]
pub struct ObsRefChildBase<Root, T, Assoc: ObsRefAssoc> {
    // self.root.value contains the real value
    // We don't store a lifetime because it's dependent on root, which is reference-counted.
    // However, we guarantee it won't be dangling.
    child_value: *mut T,
    pending: Rc<ObsRefPending<Assoc>>,
    // The child clones its pending instances and sends them to parents. This ensures that
    // when the parent gets modified, the child is still observed.
    //
    // The child also runs parent's direct pending values. This ensures that
    // when the child gets modified, the parent is still observed.
    //
    // The root can't be dropped but intermediate parents can.
    // In that case, their Weak reference will be empty, but we can just ignore them
    // since you cannot modify them specifically.
    parents_pending: Vec<Weak<ObsRefPending<Assoc>>>,
    path: String,
    root: Rc<ObsRefRootBase<Root, Assoc>>,
}

#[derive(Debug)]
struct ObsRefPending<Assoc: ObsRefAssoc> {
    pub direct: RefCell<SmallVec<[Assoc; 3]>>,
    // Child's pending values are also stored in each parent. This ensures that when the parent
    // gets modified, the child is still observed.
    pub from_children: RefCell<SmallVec<[Assoc; 3]>>,
}

// endregion
// region traits

pub trait ObsRefableRoot<Assoc: ObsRefAssoc>: Sized {
    type ObsRefImpl : ObsRef<Self, Self, Assoc>;

    fn into_obs_ref(self: Self) -> Self::ObsRefImpl;
}

pub trait ObsRefableChild<Root, Assoc: ObsRefAssoc>: Sized {
    type ObsRefImpl : ObsRef<Root, Self, Assoc>;

    unsafe fn _as_obs_ref_child(this: *mut Self, path: String, root: Weak<ObsRefRootBase<Root, Assoc>>) -> Self::ObsRefImpl;

    unsafe fn as_obs_ref_child(&mut self, path_head: &str, extension: &str, root: Weak<ObsRefRootBase<Root, Assoc>>) -> Self::ObsRefImpl {
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

impl <T, Assoc: ObsRefAssoc> ObsRefRootBase<T, Assoc> {
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

    fn send_update(&self, parents_pending: &Vec<Weak<ObsRefPending<Assoc>>>, path: &str) {
        let pending: Vec<Assoc> = parents_pending
            .iter()
            .filter_map(Weak::upgrade)
            .map(|pending| pending.direct.borrow_mut().drain(..))
            .chain(self.pending.from_children.borrow_mut().drain(..))
            .collect();

        for observer in root.observers.borrow().iter() {
            observer(&self.root_value, &pending, path)
        }
    }

    fn push_pending(&self, assoc: Assoc::Input) {
        let assoc = Assoc::from_obs_ref_assoc_input(assoc);
        self.pending.direct.borrow_mut().push(assoc);
    }
}

impl <Root, T, Assoc: ObsRefAssoc> ObsRefChildBase<Root, T, Assoc> {
    pub fn new(
        child_value: *mut T,
        ancestors_pending: &[Weak<ObsRefPending<Assoc>>],
        parent_pending: &Rc<ObsRefPending<Assoc>>,
        path: String,
        root: Rc<ObsRefRootBase<Root, Assoc>>
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

    fn push_pending(&self, assoc: Assoc::Input) {
        let assoc = Assoc::from_obs_ref_assoc_input(assoc);
        for parent in self.parents_pending {
            if let Some(parent) = parent.upgrade() {
                parent.from_children.borrow_mut().push(assoc.clone());
            }
        }
        self.pending.direct.borrow_mut().push(assoc);
    }
}

impl <Assoc: ObsRefAssoc> ObsRefPending<Assoc> {
    pub fn new() -> Self {
        ObsRefPending {
            direct: RefCell::new(SmallVec::new()),
            from_children: RefCell::new(SmallVec::new())
        }
    }
}

impl <T, Assoc: ObsRefAssoc> ObsRef<T, T, Assoc> for Rc<ObsRefRootBase<T, Assoc>> {
    fn i(&self, assoc: Assoc::Input) -> &T {
        self.push_pending(assoc);

        &self.root_value
    }

    fn m(&mut self, assoc: Assoc::Input) -> ObsDeref<T, T, Assoc> {
        self.push_pending(assoc);

        let value = &mut self.root_value as *mut T;
        ObsDeref {
            value,
            parents_pending: &Vec::new(),
            path: "",
            root: self.clone(),
        }
    }

    fn after_mutate(&self, observer: Observer<T, Assoc>) {
        self.deref().observers.borrow_mut().push(observer)
    }

    fn base(&self) -> &Rc<ObsRefRootBase<T, Assoc>> {
        self
    }
}

impl <Root, T, Assoc: ObsRefAssoc> ObsRef<Root, T, Assoc> for ObsRefChildBase<Root, T, Assoc> {
    fn i(&self, assoc: Assoc::Input) -> &T {
        self.push_pending(assoc);

        unsafe { &*self.child_value }
    }

    fn m(&mut self, assoc: Assoc::Input) -> ObsDeref<Root, T, Assoc> {
        self.push_pending(assoc);

        ObsDeref {
            value: unsafe { &mut *self.child_value },
            parents_pending: &self.parents_pending,
            path: &self.path,
            root: self.root.clone(),
        }
    }

    fn after_mutate(&self, observer: Observer<Root, Assoc>) {
        let root = self.root.upgrade().expect("ObsRefableRoot weak ref is null");
        root.observers.borrow_mut().push(observer);
    }

    #[allow(clippy::needless_lifetimes)]
    fn base<'a>(&'a self) -> &'a Rc<ObsRefRootBase<Root, Assoc>> {
        &self.root
    }
}

impl <'a, Root, T, Assoc: ObsRefAssoc> Deref for ObsDeref<'a, Root, T, Assoc> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

impl <'a, Root, T, Assoc: ObsRefAssoc> DerefMut for ObsDeref<'a, Root, T, Assoc> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl <'a, Root, T, Assoc: ObsRefAssoc> Drop for ObsDeref<'a, Root, T, Assoc> {
    fn drop(&mut self) {
        let root: &ObsRefRootBase<Root, Assoc> = unsafe {
            self.root.as_ref().expect("ObsDeref root pointer is null")
        };
        root.send_update(self.parents_pending, self.path);
    }
}

// endregion
// region boilerplate impls

impl <T: Debug, Assoc: ObsRefAssoc + Debug> Debug for ObsRefRootBase<T, Assoc> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ObsRefRootBase")
            .field("root_value", &self.root_value)
            .field("#observers", &self.observers.borrow().len())
            .field("pending", &self.pending)
            .finish()
    }
}

impl <T: PartialEq, Assoc: ObsRefAssoc> PartialEq for ObsRefRootBase<T, Assoc> {
    fn eq(&self, other: &Self) -> bool {
        self.root_value() == other.root_value()
    }
}

impl <T: PartialOrd, Assoc: ObsRefAssoc> PartialOrd for ObsRefRootBase<T, Assoc> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.root_value().partial_cmp(&other.root_value())
    }
}

impl <T: Hash, Assoc: ObsRefAssoc> Hash for ObsRefRootBase<T, Assoc> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.root_value().hash(state)
    }
}

impl <Root, T: PartialEq, Assoc: ObsRefAssoc> PartialEq for ObsRefChildBase<Root, T, Assoc> {
    fn eq(&self, other: &Self) -> bool {
        self.child_value() == other.child_value()
    }
}

impl <Root, T: PartialOrd, Assoc: ObsRefAssoc> PartialOrd for ObsRefChildBase<Root, T, Assoc> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.child_value().partial_cmp(&other.child_value())
    }
}

impl <Root, T: Hash, Assoc: ObsRefAssoc> Hash for ObsRefChildBase<Root, T, Assoc> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.child_value().hash(state)
    }
}
// endregion