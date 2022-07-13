use std::cell::RefCell;

/// Vector which you can append items to with a shared reference,
/// and take all items with a shared reference.
/// This uses `RefCell` internally but is guaranteed to never have a borrow error,
/// so it externally behaves more like `Cell`.
pub struct CellVec<T>(RefCell<Vec<T>>);

impl<T> CellVec<T> {
    pub fn new() -> Self {
        Self(RefCell::new(Vec::new()))
    }

    pub fn push(&self, item: T) {
        self.0.borrow_mut().push(item);
    }

    pub fn is_empty(&self) -> bool {
        self.0.borrow().is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.borrow().len()
    }

    pub fn take(&self) -> Vec<T> {
        self.0.take()
    }

    pub fn be_appended_to(&self, other: &mut Vec<T>) {
        other.append(&mut self.0.borrow_mut());
    }
}