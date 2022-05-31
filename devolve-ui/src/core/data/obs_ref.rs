use std::rc::{Rc, Weak};
use std::cell::RefCell;
use std::ops::{Deref, DerefMut, Index, IndexMut};

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
    // have a mutable reference to self.value, which is also in self.root.value
    // We don't alias because we only use root when dropped, and don't use value then
    root: *const ObsRefRootBase<Root>
}

pub struct ObsRefRootBase<T> {
    root_value: T,
    observers: RefCell<Vec<Observer<T>>>
}

pub struct ObsRefChildBase<Root, T> {
    // This is a pointer because self.root.value contains the real value
    child_value: *mut T,
    path: String,
    root: Weak<ObsRefRootBase<Root>>
}

pub trait ObsRefableRoot: Sized {
    type ObsRefImpl : ObsRef<Self, Self>;

    fn to_obs_ref(self: Self) -> Self::ObsRefImpl;
}

pub trait ObsRefableChild<Root>: Sized {
    type ObsRefImpl : ObsRef<Root, Self>;

    unsafe fn _to_obs_ref(self: *mut Self, path: String, root: Weak<ObsRefRootBase<Root>>) -> Self::ObsRefImpl;

    unsafe fn to_obs_ref(&self, path_head: &str, extension: &str, root: Weak<ObsRefRootBase<Root>>) -> Self::ObsRefImpl {
        let path = if extension.starts_with('[') {
            format!("{}{}", path_head, extension)
        } else {
            format!("{}.{}", path_head, extension)
        };
        ((self as *const Self) as *mut Self)._to_obs_ref(path, root)
    }
}

impl <T> ObsRefRootBase<T> {
    pub fn new(root_value: T) -> Rc<Self> {
        Rc::new(Self {
            root_value,
            observers: RefCell::new(Vec::new())
        })
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

// Specific ObsRefable implementations
pub trait Leaf {}

// impl <T> Leaf for T where T: Copy {}
impl Leaf for u8 {}
impl Leaf for u16 {}
impl Leaf for u32 {}
impl Leaf for u64 {}
impl Leaf for u128 {}
impl Leaf for i8 {}
impl Leaf for i16 {}
impl Leaf for i32 {}
impl Leaf for i64 {}
impl Leaf for i128 {}
impl Leaf for f32 {}
impl Leaf for f64 {}
impl Leaf for bool {}
impl Leaf for char {}

impl <T : Leaf> ObsRefableRoot for T {
    type ObsRefImpl = Rc<ObsRefRootBase<T>>;

    fn to_obs_ref(self: Self) -> Self::ObsRefImpl {
        ObsRefRootBase::new(self)
    }
}

impl <Root, T : Leaf> ObsRefableChild<Root> for T {
    type ObsRefImpl = ObsRefChildBase<Root, T>;

    unsafe fn _to_obs_ref(self: *mut Self, path: String, root: Weak<ObsRefRootBase<Root>>) -> Self::ObsRefImpl {
        ObsRefChildBase {
            child_value: self,
            path,
            root
        }
    }
}

pub struct ObsRefVecRoot<T : ObsRefableChild<Vec<T>>> {
    base: Rc<ObsRefRootBase<Vec<T>>>,
    children: RefCell<Vec<Option<T::ObsRefImpl>>>
}

pub struct ObsRefVecChild<Root, T : ObsRefableChild<Root>> {
    base: ObsRefChildBase<Root, Vec<T>>,
    children: RefCell<Vec<Option<T::ObsRefImpl>>>
}

impl <T : ObsRefableChild<Vec<T>>> ObsRef<Vec<T>, Vec<T>> for ObsRefVecRoot<T> {
    fn i(&self) -> &Vec<T> {
        &self.base.i()
    }

    fn m(&mut self) -> ObsDeref<Vec<T>, Vec<T>> {
        self.base.m()
    }

    fn after_mutate(&self, observer: Observer<Vec<T>>) {
        self.base.after_mutate(observer)
    }

    fn base(&self) -> Weak<ObsRefRootBase<Vec<T>>> {
        self.base.base()
    }
}

impl <Root, T : ObsRefableChild<Root>> ObsRef<Root, Vec<T>> for ObsRefVecChild<Root, T> {
    fn i(&self) -> &Vec<T> {
        &self.base.i()
    }

    fn m(&mut self) -> ObsDeref<Root, Vec<T>> {
        self.base.m()
    }

    fn after_mutate(&self, observer: Observer<Root>) {
        self.base.after_mutate(observer)
    }

    fn base(&self) -> Weak<ObsRefRootBase<Root>> {
        self.base.base()
    }
}

impl <T : ObsRefableChild<Vec<T>>> ObsRefableRoot for Vec<T> {
    type ObsRefImpl = ObsRefVecRoot<T>;

    fn to_obs_ref(self: Vec<T>) -> Self::ObsRefImpl {
        ObsRefVecRoot {
            base: Rc::new(ObsRefRootBase {
                root_value: self,
                observers: RefCell::new(Vec::new())
            }),
            children: RefCell::new(Vec::new())
        }
    }
}

impl <Root, T : ObsRefableChild<Root>> ObsRefableChild<Root> for Vec<T> {
    type ObsRefImpl = ObsRefVecChild<Root, T>;

    unsafe fn _to_obs_ref(self: *mut Vec<T>, path: String, root: Weak<ObsRefRootBase<Root>>) -> Self::ObsRefImpl {
        ObsRefVecChild {
            base: ObsRefChildBase {
                child_value: self,
                path,
                root
            },
            children: RefCell::new(Vec::new())
        }
    }
}

impl <T : ObsRefableChild<Vec<T>>> ObsRefVecRoot<T> {
    unsafe fn index_unsafe(&self, index: usize) -> &mut T::ObsRefImpl {
        let mut children = self.children.borrow_mut();
        while children.len() <= index {
            children.push(None);
        }
        let children = self.children.as_ptr().as_mut().expect("ObsRefVecRoot children is null");
        children[index].get_or_insert_with(|| {
            let extension = format!("[{}]", index);
            self.i()[index].to_obs_ref("", &extension, self.base())
        })
    }
}

impl <T : ObsRefableChild<Vec<T>>> Index<usize> for ObsRefVecRoot<T> {
    type Output = T::ObsRefImpl;

    fn index(&self, index: usize) -> &Self::Output {
        unsafe {
            self.index_unsafe(index)
        }
    }
}

impl <T : ObsRefableChild<Vec<T>>> IndexMut<usize> for ObsRefVecRoot<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        unsafe {
            self.index_unsafe(index)
        }
    }
}

impl <Root, T : ObsRefableChild<Root>> ObsRefVecChild<Root, T> {
    unsafe fn index_unsafe(&self, index: usize) -> &mut T::ObsRefImpl {
        let mut children = self.children.borrow_mut();
        while children.len() <= index {
            children.push(None);
        }
        let children = self.children.as_ptr().as_mut().expect("ObsRefVecChild children is null");
        children[index].get_or_insert_with(|| {
            let extension = format!("[{}]", index);
            let root = self.base.root.clone();
            self.i()[index].to_obs_ref("", &extension, root)
        })
    }
}

impl <Root, T : ObsRefableChild<Root>> Index<usize> for ObsRefVecChild<Root, T> {
    type Output = T::ObsRefImpl;

    fn index(&self, index: usize) -> &Self::Output {
        unsafe {
            self.index_unsafe(index)
        }
    }
}

impl <Root, T : ObsRefableChild<Root>> IndexMut<usize> for ObsRefVecChild<Root, T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        unsafe {
            self.index_unsafe(index)
        }
    }
}