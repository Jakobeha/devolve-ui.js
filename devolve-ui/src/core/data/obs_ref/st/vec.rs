use std::rc::{Rc, Weak};
use std::cell::RefCell;
use std::ops::{Index, IndexMut};
use crate::core::data::obs_ref::st::{Observer, ObsRefableRoot, ObsRefableChild, ObsRefRootBase, ObsRefChildBase, ObsRefPending, ObsRef, ObsDeref, SubCtx};

pub struct ObsRefRootForVec<T : ObsRefableChild<Vec<T>, S>, S: SubCtx> {
    base: Rc<ObsRefRootBase<Vec<T>, S>>,
    children: RefCell<Vec<Option<T::ObsRefImpl>>>
}

pub struct ObsRefChildForVec<Root, T : ObsRefableChild<Root, S>, S: SubCtx> {
    base: ObsRefChildBase<Root, Vec<T>, S>,
    children: RefCell<Vec<Option<T::ObsRefImpl>>>
}

impl <T : ObsRefableChild<Vec<T>, S>, S: SubCtx> ObsRef<Vec<T>, Vec<T>, S> for ObsRefRootForVec<T, S> {
    fn i(&self, s: S::Input<'_>) -> &Vec<T> {
        &self.base.i(s)
    }

    fn m(&mut self, s: S::Input<'_>) -> ObsDeref<Vec<T>, Vec<T>, S> {
        self.base.m(s)
    }

    fn after_mutate(&self, observer: Observer<Vec<T>, S>) {
        self.base.after_mutate(observer)
    }

    fn base(&self) -> &Rc<ObsRefRootBase<Vec<T>, S>> {
        self.base.base()
    }
}

impl <Root, T : ObsRefableChild<Root, S>, S: SubCtx> ObsRef<Root, Vec<T>, S> for ObsRefChildForVec<Root, T, S> {
    fn i(&self, s: S::Input<'_>) -> &Vec<T> {
        &self.base.i(s)
    }

    fn m(&mut self, s: S::Input<'_>) -> ObsDeref<Root, Vec<T>, S> {
        self.base.m(s)
    }

    fn after_mutate(&self, observer: Observer<Root, S>) {
        self.base.after_mutate(observer)
    }

    fn base(&self) -> &Rc<ObsRefRootBase<Root, S>> {
        self.base.base()
    }
}

impl <T: ObsRefableChild<Vec<T>, S>, S: SubCtx> ObsRefableRoot<S> for Vec<T> {
    type ObsRefImpl = ObsRefRootForVec<T, S>;

    fn into_obs_ref(self: Vec<T>) -> Self::ObsRefImpl {
        ObsRefRootForVec {
            base: ObsRefRootBase::new(self),
            children: RefCell::new(Vec::new())
        }
    }
}

impl <Root, T: ObsRefableChild<Root, S>, S: SubCtx> ObsRefableChild<Root, S> for Vec<T> {
    type ObsRefImpl = ObsRefChildForVec<Root, T, S>;

    unsafe fn _as_obs_ref_child(
        this: *mut Vec<T>,
        ancestors_pending: &[Weak<ObsRefPending<S>>],
        parent_pending: &Rc<ObsRefPending<S>>,
        path: String,
        root: Rc<ObsRefRootBase<Root, S>>
    ) -> Self::ObsRefImpl {
        ObsRefChildForVec {
            base: ObsRefChildBase::new(this, ancestors_pending, parent_pending, path, root),
            children: RefCell::new(Vec::new())
        }
    }
}

impl <T: ObsRefableChild<Vec<T>, S>, S: SubCtx> ObsRefRootForVec<T, S> {
    unsafe fn index_unsafe(&self, index: usize) -> &mut T::ObsRefImpl {
        let mut children = self.children.borrow_mut();
        while children.len() <= index {
            children.push(None);
        }
        let children = self.children.as_ptr().as_mut().expect("ObsRefVecRoot children is null");
        children[index].get_or_insert_with(|| {
            let root = self.base().clone();
            let extension = format!("[{}]", index);
            let value = &mut self.base.root_value;
            value[index].as_obs_ref_child(
                &[],
                &self.base.pending,
                "",
                &extension,
                root
            )
        })
    }
}

impl <T: ObsRefableChild<Vec<T>, S>, S: SubCtx> Index<usize> for ObsRefRootForVec<T, S> {
    type Output = T::ObsRefImpl;

    fn index(&self, index: usize) -> &Self::Output {
        unsafe {
            self.index_unsafe(index)
        }
    }
}

impl <T: ObsRefableChild<Vec<T>, S>, S: SubCtx> IndexMut<usize> for ObsRefRootForVec<T, S> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        unsafe {
            self.index_unsafe(index)
        }
    }
}

impl <Root, T: ObsRefableChild<Root, S>, S: SubCtx> ObsRefChildForVec<Root, T, S> {
    unsafe fn index_unsafe(&self, index: usize) -> &mut T::ObsRefImpl {
        let mut children = self.children.borrow_mut();
        while children.len() <= index {
            children.push(None);
        }
        let children = self.children.as_ptr().as_mut().expect("ObsRefVecChild children is null");
        children[index].get_or_insert_with(|| {
            let extension = format!("[{}]", index);
            let root = self.base().clone();
            let value = &mut *self.base.child_value;
            value[index].as_obs_ref_child(
                &self.base.parents_pending,
                &self.base.pending,
                "",
                &extension,
                root
            )
        })
    }
}

impl <Root, T: ObsRefableChild<Root, S>, S: SubCtx> Index<usize> for ObsRefChildForVec<Root, T, S> {
    type Output = T::ObsRefImpl;

    fn index(&self, index: usize) -> &Self::Output {
        unsafe {
            self.index_unsafe(index)
        }
    }
}

impl <Root, T: ObsRefableChild<Root, S>, S: SubCtx> IndexMut<usize> for ObsRefChildForVec<Root, T, S> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        unsafe {
            self.index_unsafe(index)
        }
    }
}