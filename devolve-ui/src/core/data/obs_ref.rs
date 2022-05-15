use std::rc::{Rc, Weak};
use std::cell::RefCell;
use std::ops::{Deref, DerefMut};

pub type Observer<Root> = impl Fn(&Root, &str) -> ();

/// Holds a mutable reference. Whenever you access it mutably,
/// it will trigger observers. You can access an observable reference to children of `ObsRefable`
pub trait ObsRef<Root, T> {
    /// Returns an immutable reference to the underlying value
    fn i(&self) -> &T;

    /// Returns a mutable reference to the underlying value.
    /// When the reference is dropped, observers will be called
    fn m(&mut self) -> ObsDeref<T>;

    /// Add observer which will be called when `m` is called and then the reference is dropped.
    fn after_mutate(&self, observer: Observer<Root>);

    fn base(&self) -> &Weak<ObsRefRootBase<Root>>;
}

struct ObsDeref<'a, T> {
    value: &'a mut T,
    path: &'a str,
    root: Weak<ObsRefRootBase<T>>
}

pub struct ObsRefRootBase<T> {
    value: T,
    observers: RefCell<Vec<Observer<T>>>
}

pub struct ObsRefChildBase<Root, T> {
    value: *mut T,
    path: String,
    root: Weak<ObsRefRootBase<Root>>
}

pub trait ObsRefableRoot: Sized {
    type ObsRefImpl : ObsRef<Self, Self>;

    fn to_obs_ref(self: Self) -> Self::ObsRefImpl;
}

pub trait ObsRefableChild<Root>: Sized {
    type ObsRefImpl : ObsRef<Root, Self>;

    fn _to_obs_ref(self: *mut Self, path: String, root: Weak<ObsRefRootBase<Root>>) -> Self::ObsRefImpl;

    fn to_obs_ref(self: *mut Self, path_head: &str, extension: &str, root: impl ObsRef<Root, Root>) -> Self::ObsRefImpl {
        let path = format!("{}.{}", path_head, extension);
        self._to_obs_ref(path, path, root.base().clone())
    }
}

impl <T> ObsRef<T, T> for Weak<ObsRefRootBase<T>> {
    fn i(&self) -> &T {
        &self.value
    }

    fn m(&mut self) -> ObsDeref<T> {
        ObsDeref {
            value: &mut self.value,
            path: "",
            root: self.clone()
        }
    }

    fn after_mutate(&self, observer: Observer<T>) {
        self.observers.borrow_mut().push(observer);
    }

    fn base(&self) -> &Weak<ObsRefRootBase<T>> {
        self
    }
}

impl <'a, T> Deref for ObsDeref<'a, T> {
    type Target = T;

    fn deref(&self) -> &'a T {
        &self.value
    }
}

impl <'a, T> DerefMut for ObsDeref<'a, T> {
    fn deref_mut(&mut self) -> &'a mut T {
        &mut self.value
    }
}

impl <'a, T> Drop for ObsDeref<'a, T> {
    fn drop(&mut self) {
        if let Some(root) = self.root.upgrade() {
            for observer in root.observers.borrow().iter() {
                observer(&root.value, &self.path)
            }
        }
    }
}

// Specific ObsRefable implementations
pub trait Leaf {}

// impl <T> Leaf for T where T: Copy {}
impl Leaf for u32 {}

impl <T : Leaf> ObsRefableRoot for T {
    type ObsRefImpl = Rc<ObsRefRootBase<T>>;

    fn to_obs_ref(self: Self) -> Self::ObsRefImpl {
        Rc::new(ObsRefRootBase {
            value: self,
            observers: RefCell::new(Vec::new())
        })
    }
}

impl <Root, T : Leaf> ObsRefableChild<Root> for T {
    type ObsRefImpl = ObsRefChildBase<Root, T>;

    fn _to_obs_ref(self: *mut Self, path: String, root: Weak<ObsRefRootBase<T>>) -> Self::ObsRefImpl {
        ObsRefChildBase {
            value: self,
            path,
            root
        }
    }
}

pub struct ObsRefVecRoot<T>(Rc<ObsRefRootBase<Vec<T>>>);

pub struct ObsRefVecChild<Root, T>(ObsRefChildBase<Root, Vec<T>>);

impl <T> ObsRef<Vec<T>, Vec<T>> for ObsRefVecRoot<T> {
    fn i(&self) -> &Vec<T> {
        &self.0.i()
    }

    fn m(&mut self) -> ObsDeref<Vec<T>> {
        self.0.m()
    }

    fn after_mutate(&self, observer: Observer<Vec<T>>) {
        self.0.after_mutate(observer)
    }

    fn base(&self) -> &Weak<ObsRefRootBase<Vec<T>>> {
        &self.0
    }
}

impl <Root, T> ObsRef<Root, Vec<T>> for ObsRefVecChild<Root, T> {
    fn i(&self) -> &Root {
        &self.0.i()
    }

    fn m(&mut self) -> ObsDeref<Root> {
        self.0.m()
    }

    fn after_mutate(&self, observer: Observer<Root>) {
        self.0.after_mutate(observer)
    }

    fn base(&self) -> &Weak<ObsRefRootBase<Root>> {
        &self.0.base()
    }
}

impl <T : ObsRefableChild<Vec<T>>> ObsRefableRoot for Vec<T> {
    type ObsRefImpl = ObsRefVecRoot<T>;

    fn to_obs_ref(self: Vec<T>) -> Self::ObsRefImpl {
        ObsRefVecRoot(Rc::new(ObsRefRootBase {
            value: self,
            observers: RefCell::new(Vec::new())
        }))
    }
}

impl <Root, T : ObsRefableChild<Vec<T>>> ObsRefableChild<Root> for Vec<T> {
    type ObsRefImpl = ObsRefVecChild<Root, T>;

    fn _to_obs_ref(self: *mut Vec<T>, path: String, root: Weak<ObsRefRootBase<Root>>) -> Self::ObsRefImpl {
        ObsRefVecChild(ObsRefChildBase {
            value: self,
            path,
            root
        })
    }
}