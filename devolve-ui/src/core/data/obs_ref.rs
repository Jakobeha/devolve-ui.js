use std::rc::{Rc, Weak};
use std::cell::RefCell;
use std::ops::{Deref, DerefMut};

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

pub struct ObsDeref<'a, Root, T> {
    value: &'a mut T,
    path: &'a str,
    // This is a pointer not a Weak reference, because we
    // have a mutable reference to value.
    // We don't alias because we only use root when value is dropped
    root: *const ObsRefRootBase<Root>
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
        self._to_obs_ref(path, root.base().clone())
    }
}

/* impl <T> ObsRef<T, T> for Weak<ObsRefRootBase<T>> {
    fn i(&self) -> &T {
        let as_ref: &ObsRefRootBase<T>;
        unsafe {
            as_ref = self.as_ptr().as_ref().expect("ObsRefableRoot weak ref is null");
        }
        &as_ref.value
    }

    fn m(&mut self) -> ObsDeref<T, T> {
        let mut as_rc: Rc<ObsRefRootBase<T>> = self.upgrade().expect("ObsRefableRoot weak ref is null");
        let as_mut: &mut ObsRefRootBase<T> = Rc::get_mut(&mut as_rc).expect("ObsRefableRoot borrowed multiple times");
        let root = as_mut as *const _;
        ObsDeref {
            value: &mut as_mut.value,
            path: "",
            root
        }
    }

    fn after_mutate(&self, observer: Observer<T>) {
        let as_ref: &ObsRefRootBase<T>;
        unsafe {
            as_ref = self.as_ptr().as_ref().expect("ObsRefableRoot weak ref is null");
        }
        as_ref.observers.borrow_mut().push(observer);
    }

    fn base(&self) -> Weak<ObsRefRootBase<T>> {
        self.clone()
    }
} */

impl <T> ObsRef<T, T> for Rc<ObsRefRootBase<T>> {
    fn i(&self) -> &T {
        &self.value
    }

    fn m(&mut self) -> ObsDeref<T, T> {
        let as_mut: &mut ObsRefRootBase<T> = Rc::get_mut(self).expect("ObsRefableRoot borrowed multiple times");
        let root = as_mut as *const _;
        ObsDeref {
            value: &mut as_mut.value,
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
            self.value.as_ref().expect("ObsRef child pointer is null")
        }
    }

    fn m(&mut self) -> ObsDeref<Root, T> {
        unsafe {
            ObsDeref {
                value: self.value.as_mut().expect("ObsRef child pointer is null"),
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
            observer(&root.value, &self.path)
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

    fn _to_obs_ref(self: *mut Self, path: String, root: Weak<ObsRefRootBase<Root>>) -> Self::ObsRefImpl {
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

    fn m(&mut self) -> ObsDeref<Vec<T>, Vec<T>> {
        self.0.m()
    }

    fn after_mutate(&self, observer: Observer<Vec<T>>) {
        self.0.after_mutate(observer)
    }

    fn base(&self) -> Weak<ObsRefRootBase<Vec<T>>> {
        self.0.base()
    }
}

impl <Root, T> ObsRef<Root, Vec<T>> for ObsRefVecChild<Root, T> {
    fn i(&self) -> &Vec<T> {
        &self.0.i()
    }

    fn m(&mut self) -> ObsDeref<Root, Vec<T>> {
        self.0.m()
    }

    fn after_mutate(&self, observer: Observer<Root>) {
        self.0.after_mutate(observer)
    }

    fn base(&self) -> Weak<ObsRefRootBase<Root>> {
        self.0.base()
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