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

pub type Observer<Root> = Box<dyn Fn(&Root, &str) -> ()>;

/// Holds a mutable reference. Whenever you access it mutably,
/// it will trigger observers. You can access an observable reference to children of `ObsRefable`
pub trait ObsRef<Root, T> {
    /// Returns an immutable reference to the underlying value
    fn i(&self) -> &T;

    /// Returns a mutable reference to the underlying value.
    /// When the reference is dropped, observers will be called
    fn m(&mut self) -> ObsDeref<Root, T>;

    /// Add observer which will be called when `m` is called and then the reference is dropped.
    fn after_mutate(&self, observer: Observer<Root>);

    fn base(&self) -> Weak<ObsRefRootBase<Root>>;
}

#[derive(Debug)]
pub struct ObsDeref<'a, Root, T> {
    value: &'a mut T,
    path: &'a str,
    // This is a pointer not a Weak reference, because we
    // have a mutable reference to self.value, which is also in self.root.value
    // We don't alias because we only use root when dropped, and don't use value then
    root: *const ObsRefRootBase<Root>
}

// TODO: Do we need to pin Rc<ObsRefRootBase<T>>?
//   The children child_values reference root_value,
//   but they should't outlive root_value and shouldn't exist
//   if root_value has mutable access (since they also have mutable access)
pub struct ObsRefRootBase<T> {
    root_value: T,
    // SmallVec is ideal here because we usually don't have many observers
    observers: RefCell<SmallVec<[Observer<T>; 3]>>
}

#[derive(Debug)]
pub struct ObsRefChildBase<Root, T> {
    // This is a pointer because self.root.value contains the real value
    child_value: *mut T,
    path: String,
    root: Weak<ObsRefRootBase<Root>>
}

// endregion
// region traits

pub trait ObsRefableRoot: Sized {
    type ObsRefImpl : ObsRef<Self, Self>;

    fn into_obs_ref(self: Self) -> Self::ObsRefImpl;
}

pub trait ObsRefableChild<Root>: Sized {
    type ObsRefImpl : ObsRef<Root, Self>;

    unsafe fn _as_obs_ref_child(this: *mut Self, path: String, root: Weak<ObsRefRootBase<Root>>) -> Self::ObsRefImpl;

    unsafe fn as_obs_ref_child(&self, path_head: &str, extension: &str, root: Weak<ObsRefRootBase<Root>>) -> Self::ObsRefImpl {
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

impl <T> ObsRefRootBase<T> {
    pub fn new(root_value: T) -> Rc<Self> {
        Rc::new(Self {
            root_value,
            observers: RefCell::new(SmallVec::new())
        })
    }

    pub fn root_value(&self) -> &T {
        &self.root_value
    }
}

impl <Root, T> ObsRefChildBase<Root, T> {
    pub fn new(child_value: *mut T, path: String, root: Weak<ObsRefRootBase<Root>>) -> Self {
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

impl <T> ObsRef<T, T> for Rc<ObsRefRootBase<T>> {
    fn i(&self) -> &T {
        &self.root_value
    }

    fn m(&mut self) -> ObsDeref<T, T> {
        let as_mut: &mut ObsRefRootBase<T> = Rc::get_mut(self).expect("ObsRefableRoot borrowed multiple times");
        let root = as_mut as *const _;
        ObsDeref {
            value: &mut as_mut.root_value,
            path: "",
            root
        }
    }

    fn after_mutate(&self, observer: Observer<T>) {
        self.deref().observers.borrow_mut().push(observer)
    }

    fn base(&self) -> Weak<ObsRefRootBase<T>> {
        Rc::downgrade(self)
    }
}

impl <Root, T> ObsRef<Root, T> for ObsRefChildBase<Root, T> {
    fn i(&self) -> &T {
        unsafe {
            self.child_value.as_ref().expect("ObsRef child pointer is null")
        }
    }

    fn m(&mut self) -> ObsDeref<Root, T> {
        unsafe {
            ObsDeref {
                value: self.child_value.as_mut().expect("ObsRef child pointer is null"),
                path: &self.path,
                root: self.root.as_ptr()
            }
        }
    }

    fn after_mutate(&self, observer: Observer<Root>) {
        let root_ref: &ObsRefRootBase<Root>;
        unsafe {
            root_ref = self.root.as_ptr().as_ref().expect("ObsRefableRoot weak ref is null");
        }
        root_ref.observers.borrow_mut().push(observer);
    }

    fn base(&self) -> Weak<ObsRefRootBase<Root>> {
        self.root.clone()
    }
}

impl <'a, Root, T> Deref for ObsDeref<'a, Root, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

impl <'a, Root, T> DerefMut for ObsDeref<'a, Root, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl <'a, Root, T> Drop for ObsDeref<'a, Root, T> {
    fn drop(&mut self) {
        let root: &ObsRefRootBase<Root>;
        unsafe {
            root = self.root.as_ref().expect("ObsDeref root pointer is null");
        }
        for observer in root.observers.borrow().iter() {
            observer(&root.root_value, &self.path)
        }
    }
}

// endregion
// region boilerplate impls

impl <T: Debug> Debug for ObsRefRootBase<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "ObsRefRootBase({:?})", self.root_value)
    }
}

impl <T: PartialEq> PartialEq for ObsRefRootBase<T> {
    fn eq(&self, other: &Self) -> bool {
        self.root_value() == other.root_value()
    }
}

impl <T: PartialOrd> PartialOrd for ObsRefRootBase<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.root_value().partial_cmp(&other.root_value())
    }
}

impl <T: Hash> Hash for ObsRefRootBase<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.root_value().hash(state)
    }
}

impl <Root, T: PartialEq> PartialEq for ObsRefChildBase<Root, T> {
    fn eq(&self, other: &Self) -> bool {
        self.child_value() == other.child_value()
    }
}

impl <Root, T: PartialOrd> PartialOrd for ObsRefChildBase<Root, T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.child_value().partial_cmp(&other.child_value())
    }
}

impl <Root, T: Hash> Hash for ObsRefChildBase<Root, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.child_value().hash(state)
    }
}

// endregion