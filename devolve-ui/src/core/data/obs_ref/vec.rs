use std::rc::{Rc, Weak};
use std::cell::RefCell;
use std::ops::{Index, IndexMut};
use crate::core::data::obs_ref::{Observer, ObsRefableRoot, ObsRefableChild, ObsRefRootBase, ObsRefChildBase, ObsRef, ObsDeref};

pub struct ObsRefRootForVec<T : ObsRefableChild<Vec<T>>> {
    base: Rc<ObsRefRootBase<Vec<T>>>,
    children: RefCell<Vec<Option<T::ObsRefImpl>>>
}

pub struct ObsRefChildForVec<Root, T : ObsRefableChild<Root>> {
    base: ObsRefChildBase<Root, Vec<T>>,
    children: RefCell<Vec<Option<T::ObsRefImpl>>>
}

impl <T : ObsRefableChild<Vec<T>>> ObsRef<Vec<T>, Vec<T>> for ObsRefRootForVec<T> {
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

impl <Root, T : ObsRefableChild<Root>> ObsRef<Root, Vec<T>> for ObsRefChildForVec<Root, T> {
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
    type ObsRefImpl = ObsRefRootForVec<T>;

    fn into_obs_ref(self: Vec<T>) -> Self::ObsRefImpl {
        ObsRefRootForVec {
            base: ObsRefRootBase::new(self),
            children: RefCell::new(Vec::new())
        }
    }
}

impl <Root, T : ObsRefableChild<Root>> ObsRefableChild<Root> for Vec<T> {
    type ObsRefImpl = ObsRefChildForVec<Root, T>;

    unsafe fn _as_obs_ref_child(this: *mut Vec<T>, path: String, root: Weak<ObsRefRootBase<Root>>) -> Self::ObsRefImpl {
        ObsRefChildForVec {
            base: ObsRefChildBase {
                child_value: this,
                path,
                root
            },
            children: RefCell::new(Vec::new())
        }
    }
}

impl <T : ObsRefableChild<Vec<T>>> ObsRefRootForVec<T> {
    unsafe fn index_unsafe(&self, index: usize) -> &mut T::ObsRefImpl {
        let mut children = self.children.borrow_mut();
        while children.len() <= index {
            children.push(None);
        }
        let children = self.children.as_ptr().as_mut().expect("ObsRefVecRoot children is null");
        children[index].get_or_insert_with(|| {
            let extension = format!("[{}]", index);
            self.i()[index].as_obs_ref_child("", &extension, self.base())
        })
    }
}

impl <T : ObsRefableChild<Vec<T>>> Index<usize> for ObsRefRootForVec<T> {
    type Output = T::ObsRefImpl;

    fn index(&self, index: usize) -> &Self::Output {
        unsafe {
            self.index_unsafe(index)
        }
    }
}

impl <T : ObsRefableChild<Vec<T>>> IndexMut<usize> for ObsRefRootForVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        unsafe {
            self.index_unsafe(index)
        }
    }
}

impl <Root, T : ObsRefableChild<Root>> ObsRefChildForVec<Root, T> {
    unsafe fn index_unsafe(&self, index: usize) -> &mut T::ObsRefImpl {
        let mut children = self.children.borrow_mut();
        while children.len() <= index {
            children.push(None);
        }
        let children = self.children.as_ptr().as_mut().expect("ObsRefVecChild children is null");
        children[index].get_or_insert_with(|| {
            let extension = format!("[{}]", index);
            let root = self.base.root.clone();
            self.i()[index].as_obs_ref_child("", &extension, root)
        })
    }
}

impl <Root, T : ObsRefableChild<Root>> Index<usize> for ObsRefChildForVec<Root, T> {
    type Output = T::ObsRefImpl;

    fn index(&self, index: usize) -> &Self::Output {
        unsafe {
            self.index_unsafe(index)
        }
    }
}

impl <Root, T : ObsRefableChild<Root>> IndexMut<usize> for ObsRefChildForVec<Root, T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        unsafe {
            self.index_unsafe(index)
        }
    }
}