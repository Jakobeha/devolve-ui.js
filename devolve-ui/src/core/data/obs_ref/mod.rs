// TODO: Explain (+ explain Assoc and you can set it to () if you don't want it)

pub mod leaf;
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
pub trait ObsRef<Root, T, Assoc> {
    /// Returns an immutable reference to the underlying value
    fn i(&self, assoc: Assoc) -> &T;

    /// Returns a mutable reference to the underlying value.
    /// When the reference is dropped, observers will be called
    fn m(&mut self, assoc: Assoc) -> ObsDeref<Root, T, Assoc>;

    /// Add observer which will be called when `m` is called and then the reference is dropped.
    fn after_mutate(&self, observer: Observer<Root, Assoc>);

    fn base(&self) -> Weak<ObsRefRootBase<Root, Assoc>>;
}

#[derive(Debug)]
pub struct ObsDeref<'a, Root, T, Assoc> {
    value: &'a mut T,
    path: &'a str,
    // This is a pointer not a Weak reference, because we
    // have a mutable reference to self.value, which is also in self.root.value
    // We don't alias because we only use root when dropped, and don't use value then
    root: *const ObsRefRootBase<Root, Assoc>
}

// TODO: Do we need to pin Rc<ObsRefRootBase<T>>?
//   The children child_values reference root_value,
//   but they should't outlive root_value and shouldn't exist
//   if root_value has mutable access (since they also have mutable access)
pub struct ObsRefRootBase<T, Assoc> {
    root_value: T,
    // SmallVec is ideal here because we usually don't have many observers
    observers: RefCell<SmallVec<[Observer<T, Assoc>; 3]>>,
    pending: RefCell<SmallVec<[Assoc; 3]>>
}

#[derive(Debug)]
pub struct ObsRefChildBase<Root, T, Assoc> {
    // This is a pointer because self.root.value contains the real value
    child_value: *mut T,
    path: String,
    root: Weak<ObsRefRootBase<Root, Assoc>>
}

// endregion
// region traits

pub trait ObsRefableRoot<Assoc>: Sized {
    type ObsRefImpl : ObsRef<Self, Self, Assoc>;

    fn into_obs_ref(self: Self) -> Self::ObsRefImpl;
}

pub trait ObsRefableChild<Root, Assoc>: Sized {
    type ObsRefImpl : ObsRef<Root, Self, Assoc>;

    unsafe fn _as_obs_ref_child(this: *mut Self, path: String, root: Weak<ObsRefRootBase<Root, Assoc>>) -> Self::ObsRefImpl;

    unsafe fn as_obs_ref_child(&self, path_head: &str, extension: &str, root: Weak<ObsRefRootBase<Root, Assoc>>) -> Self::ObsRefImpl {
        let path = if extension.starts_with('[') {
            format!("{}{}", path_head, extension)
        } else {
            format!("{}.{}", path_head, extension)
        };
        Self::_as_obs_ref_child((self as *const Self) as *mut Self, path, root)
    }
}

// endregion
// region main impls

impl <T, Assoc> ObsRefRootBase<T, Assoc> {
    pub fn new(root_value: T) -> Rc<Self> {
        Rc::new(Self {
            root_value,
            observers: RefCell::new(SmallVec::new()),
            pending: RefCell::new(SmallVec::new())
        })
    }

    pub fn root_value(&self) -> &T {
        &self.root_value
    }

    fn send_update(&self, path: &str) {
        let pending = self.pending.borrow();
        for observer in root.observers.borrow().iter() {
            observer(&self.root_value, pending.as_ref(), path)
        }
        // Really make sure this is dropped before borrow_mut
        drop(pending);
        self.pending.borrow_mut().clear()
    }
}

impl <Root, T, Assoc> ObsRefChildBase<Root, T, Assoc> {
    pub fn new(child_value: *mut T, path: String, root: Weak<ObsRefRootBase<Root, Assoc>>) -> Self {
        Self {
            child_value,
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
}

impl <T, Assoc> ObsRef<T, T, Assoc> for Rc<ObsRefRootBase<T, Assoc>> {
    fn i(&self, assoc: Assoc) -> &T {
        self.pending.get_mut().push(assoc);

        &self.root_value
    }

    fn m(&mut self, assoc: Assoc) -> ObsDeref<T, T, Assoc> {
        self.pending.get_mut().push(assoc);

        let as_mut: &mut ObsRefRootBase<T, Assoc> = Rc::get_mut(self).expect("ObsRefableRoot borrowed multiple times");
        let root = as_mut as *const _;
        ObsDeref {
            value: &mut as_mut.root_value,
            path: "",
            root
        }
    }

    fn after_mutate(&self, observer: Observer<T, Assoc>) {
        self.deref().observers.borrow_mut().push(observer)
    }

    fn base(&self) -> Weak<ObsRefRootBase<T, Assoc>> {
        Rc::downgrade(self)
    }
}

impl <Root, T, Assoc> ObsRef<Root, T, Assoc> for ObsRefChildBase<Root, T, Assoc> {
    fn i(&self, assoc: Assoc) -> &T {
        let root = self.root.upgrade().expect("ObsRefableRoot weak ref is null");
        root.pending.borrow_mut().push(assoc);

        unsafe {
            self.child_value.as_ref().expect("ObsRef child pointer is null")
        }
    }

    fn m(&mut self, assoc: Assoc) -> ObsDeref<Root, T, Assoc> {
        let root = self.root.upgrade().expect("ObsRefableRoot weak ref is null");
        root.pending.borrow_mut().push(assoc);

        unsafe {
            ObsDeref {
                value: self.child_value.as_mut().expect("ObsRef child pointer is null"),
                path: &self.path,
                root: self.root.as_ptr()
            }
        }
    }

    fn after_mutate(&self, observer: Observer<Root, Assoc>) {
        let root = self.root.upgrade().expect("ObsRefableRoot weak ref is null");
        root.observers.borrow_mut().push(observer);
    }

    fn base(&self) -> Weak<ObsRefRootBase<Root, Assoc>> {
        self.root.clone()
    }
}

impl <'a, Root, T, Assoc> Deref for ObsDeref<'a, Root, T, Assoc> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

impl <'a, Root, T, Assoc> DerefMut for ObsDeref<'a, Root, T, Assoc> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl <'a, Root, T, Assoc> Drop for ObsDeref<'a, Root, T, Assoc> {
    fn drop(&mut self) {
        let root: &ObsRefRootBase<Root, Assoc> = unsafe {
            self.root.as_ref().expect("ObsDeref root pointer is null")
        };
        root.send_update(self.path);
    }
}

// endregion
// region boilerplate impls

impl <T: Debug, Assoc> Debug for ObsRefRootBase<T, Assoc> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "ObsRefRootBase({:?})", self.root_value)
    }
}

impl <T: PartialEq, Assoc> PartialEq for ObsRefRootBase<T, Assoc> {
    fn eq(&self, other: &Self) -> bool {
        self.root_value() == other.root_value()
    }
}

impl <T: PartialOrd, Assoc> PartialOrd for ObsRefRootBase<T, Assoc> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.root_value().partial_cmp(&other.root_value())
    }
}

impl <T: Hash, Assoc> Hash for ObsRefRootBase<T, Assoc> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.root_value().hash(state)
    }
}

impl <Root, T: PartialEq, Assoc> PartialEq for ObsRefChildBase<Root, T, Assoc> {
    fn eq(&self, other: &Self) -> bool {
        self.child_value() == other.child_value()
    }
}

impl <Root, T: PartialOrd, Assoc> PartialOrd for ObsRefChildBase<Root, T, Assoc> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.child_value().partial_cmp(&other.child_value())
    }
}

impl <Root, T: Hash, Assoc> Hash for ObsRefChildBase<Root, T, Assoc> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.child_value().hash(state)
    }
}
// endregion