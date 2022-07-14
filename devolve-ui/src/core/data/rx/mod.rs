//! `Rx` means "reactive value" (or "reactive X"). It is a wrapper for a value which changes,
//! and these changes trigger dependencies to re-run and change themselves.
//!
//! Because of Rust's borrowing rules, you can't just have `Rx` values change arbitrarily,
//! because then references will be invalidated. Instead, when an `Rx` is updated, this update is delayed until there are no mutable references.
//! Furthermore, you cannot just get a mutable reference to an `Rx` value, you must set it to an entirely new value.
//!
//! The way it works is, there is an `RxDAG` which stores the entire dependency graph, and you can only get a reference to an `Rx` value
//! from a shared reference to the graph. The `Rx`s update when you call `RxDAG::recompute`, which requires a mutable reference.
//!
//! Furthermore, `Rx` closures must have a specific lifetime, because they may be recomputed.
//! This lifetime is annotated `'c` and the same lifetime is for every closure in an `RxDAG`.
//! value directly, instead you use an associated function like `run_rx` to access it in a closure
//! which can re-run whenever the dependency changes. You can create new `Rx`s from old ones.

use std::alloc::{alloc, Layout};
use std::cell::{Cell, Ref, RefCell};
use std::hash::Hash;
use std::marker::PhantomData;
use std::mem::{align_of, align_of_val, MaybeUninit, size_of, size_of_val};
use std::ops::{Deref, Index};
use std::ptr;
use std::ptr::NonNull;
use std::rc::{Rc, Weak};
use elsa::FrozenVec;
use smallvec::SmallVec;
use stable_deref_trait::StableDeref;
use crate::core::misc::cell_vec::CellAppendVec;
use crate::core::misc::slice_split3::SliceSplit3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RxDAGUid<'c>(usize, PhantomData<&'c ()>);

thread_local! {
    static RX_DAG_UID: Cell<usize> = Cell::new(0);
}

impl<'c> RxDAGUid<'c> {
    pub fn next() -> RxDAGUid<'c> {
        RX_DAG_UID.with(|uid_cell| {
            RxDAGUid(uid_cell.update(|uid| uid + 1), PhantomData)
        })
    }
}

pub trait RxContext<'c> {
    fn graph<'c2>(&self) -> &RxDAG<'c2> where 'c: 'c2;
}

impl<'c> RxContext<'c> for RxDAG<'c> {
    fn graph<'c2>(&self) -> &RxDAG<'c2> where 'c: 'c2 {
        self
    }
}

/// The DAG is a list of interspersed nodes and edges. The edges refer to other nodes relative to their own position.
/// Later Rxs *must* depend on earlier Rxs.
///
/// When the DAG recomputes, it simply iterates through each node and edge in order and calls `RxDAGElem::recompute`.
/// If the nodes were changed (directly or as edge output), they set their new value, and mark that they got recomputed.
/// The edges will recompute and change their output nodes if any of their inputs got recomputed.
///
/// The DAG has interior mutability, in that it can add nodes without a mutable borrow.
/// See `elsa` crate for why this is sound (though honestly the soundness argument is kinda sus).
/// `RxDAGElem` implements `Deref` and `StableDeref` but panics if it's an edge, however `Deref` is
/// only accessible internally and should never be able to reach the panic case.
///
/// Setting `Rx` values is also interior mutability, and OK because we don't use those values until `RxDAGElem::recompute`.
///
/// The DAG and refs have an ID so that you can't use one ref on another DAG, however this is checked at runtime.
/// The lifetimes are checked at compile-time though.
///
/// Currently no `Rx`s are deallocated until the entire DAG is deallocated,
/// so if you keep creating and discarding `Rx`s you will leak memory (TODO fix this?)
pub struct RxDAG<'c>(FrozenVec<RxDAGElem<'c>>, RxDAGUid<'c>);

enum RxDAGElem<'c> {
    Node(Box<Rx<'c>>),
    Edge(Box<RxEdge<'c>>)
}

type Rx<'c> = dyn RxTrait + 'c;
type RxEdge<'c> = dyn RxEdgeTrait + 'c;

trait RxTrait {
    fn post_read(&self) -> bool;

    fn recompute(&mut self);
    fn did_recompute(&self) -> bool;
    fn post_recompute(&mut self);

    unsafe fn _set_dyn(&self, ptr: *mut u8, size: usize);
}

impl dyn RxTrait {
    unsafe fn set_dyn<T>(&self, mut value: T) {
        self._set_dyn(&mut value as *mut T as *mut u8, size_of_val(&value));
    }
}

struct RxImpl<T> {
    current: T,
    next: Cell<Option<T>>,
    // Rx flags (might have same flags for a group to reduce traversing all Rxs)
    did_read: Cell<bool>,
    did_recompute: bool
}

trait RxEdgeTrait {
    fn recompute(&mut self, inputs: &[RxDAGElem], outputs: &[RxDAGElem]);
}

struct RxEdgeImpl<'c, F: FnMut(&mut Vec<usize>, &mut dyn Iterator<Item=&Rx<'c>>) + 'c> {
    // Takes current of input values (first argument) and sets next of output values (second argument).
    compute: F,
    num_outputs: usize,
    input_backwards_offsets: Vec<usize>,
}

/// Index into the DAG which will give you an `Rx` value.
/// However, to get or set the value you need a shared reference to the `DAG`.
///
/// The DAG and refs have an ID so that you can't use one ref on another DAG, however this is checked at runtime.
/// The lifetimes are checked at compile-time though.
#[derive(Debug, Clone, Copy)]
#[derivative(Clone(bound = ""))]
struct RxRef<'c, T> {
    index: usize,
    graph_id: RxDAGUid<'c>,
    phantom: PhantomData<T>
}

/// Index into the DAG which will give you an `Rx` variable.
/// However, to get or set the value you need a shared reference to the `DAG`.
/// This value is not computed from other values, instead you set it directly.
#[derive(Debug, Clone, Copy)]
#[derivative(Clone(bound = ""))]
pub struct Var<'c, T>(RxRef<'c, T>);

/// Index into the DAG which will give you a computed `Rx` value.
/// However, to get the value you need a shared reference to the `DAG`.
/// You cannot set the value because it's computed from other values.
#[derive(Debug, Clone, Copy)]
#[derivative(Clone(bound = ""))]
pub struct CRx<'c, T>(RxRef<'c, T>);

thread_local! {
    static ID: Uid = 0;
}

impl<'c> RxDAG<'c> {
    /// Create an empty DAG
    pub fn new() -> Self {
        Self(FrozenVec::new(), RxDAGUid::next())
    }

    /// Create a variable `Rx` in this DAG.
    pub fn new_var<T>(&self, init: T) -> Var<'c, T> {
        let index = self.next_index();
        let mut rx = RxImpl::new(init);
        self.0.push(RxDAGElem::Node(Box::new(rx)));
        Var(RxRef::new(self, index))
    }

    /// Create a computed `Rx` in this DAG.
    pub fn new_crx<T, F: FnMut() -> T>(&self, mut compute: F) -> CRx<'c, T> {
        let mut input_backwards_offsets = Vec::new();
        let init = self.run_compute(&mut compute, &mut input_backwards_offsets);
        let compute_edge = RxEdgeImpl::new(input_backwards_offsets, 1, move |mut input_backwards_offsets, outputs| {
            input_backwards_offsets.clear();
            let output = self.run_compute(&mut compute, &mut input_backwards_offsets);
            unsafe { outputs.next().unwrap().set_dyn(output); }
            debug_assert!(outputs.next().is_none());
        });
        self.0.push(RxDAGElem::Edge(Box::new(compute_edge)));

        let index = self.next_index();
        let rx = RxImpl::new(init);
        self.0.push(RxDAGElem::Node(Box::new(rx)));
        CRx(RxRef::new(self, index))
    }

    /// Create 2 computed `Rx` in this DAG which are created from the same function.
    pub fn new_crx2<T1, T2, F: FnMut() -> (T1, T2)>(&self, mut compute: F) -> (CRx<'c, T1>, CRx<'c, T2>) {
        let mut input_backwards_offsets = Vec::new();
        let (init1, init2) = self.run_compute(&mut compute, &mut input_backwards_offsets);
        let compute_edge = RxEdgeImpl::new(input_backwards_offsets, 2, move |mut input_backwards_offsets, outputs| {
            input_backwards_offsets.clear();
            let (output1, output2) = self.run_compute(&mut compute, &mut input_backwards_offsets);
            unsafe { outputs.next().unwrap().set_dyn(output1); }
            unsafe { outputs.next().unwrap().set_dyn(output2); }
            debug_assert!(outputs.next().is_none());
        });
        self.0.push(RxDAGElem::Edge(Box::new(compute_edge)));

        let index = self.next_index();
        let mut rx1 = RxImpl::new(init1);
        let mut rx2 = RxImpl::new(init2);
        self.0.push(RxDAGElem::Node(Box::new(rx1)));
        self.0.push(RxDAGElem::Node(Box::new(rx2)));
        (CRx(RxRef::new(self, index)), CRx(RxRef::new(self, index + 1)))
    }

    /// Create 3 computed `Rx` in this DAG which are created from the same function.
    pub fn new_crx3<T1, T2, T3, F: FnMut() -> (T1, T2, T3)>(&self, mut compute: F) -> (CRx<'c, T1>, CRx<'c, T2>, CRx<'c, T3>) {
        let mut input_backwards_offsets = Vec::new();
        let (init1, init2, init3) = self.run_compute(&mut compute, &mut input_backwards_offsets);
        let compute_edge = RxEdgeImpl::new(input_backwards_offsets, 2, move |mut input_backwards_offsets, outputs| {
            input_backwards_offsets.clear();
            let (output1, output2, output3) = self.run_compute(&mut compute, &mut input_backwards_offsets);
            unsafe { outputs.next().unwrap().set_dyn(output1); }
            unsafe { outputs.next().unwrap().set_dyn(output2); }
            unsafe { outputs.next().unwrap().set_dyn(output3); }
            debug_assert!(outputs.next().is_none());
        });
        self.0.push(RxDAGElem::Edge(Box::new(compute_edge)));

        let index = self.next_index();
        let mut rx1 = RxImpl::new(init1);
        let mut rx2 = RxImpl::new(init2);
        let mut rx3 = RxImpl::new(init3);
        self.0.push(RxDAGElem::Node(Box::new(rx1)));
        self.0.push(RxDAGElem::Node(Box::new(rx2)));
        self.0.push(RxDAGElem::Node(Box::new(rx3)));
        (CRx(RxRef::new(self, index)), CRx(RxRef::new(self, index + 1)), CRx(RxRef::new(self, index + 2)))
    }

    fn next_index(&self) -> usize {
        self.0.len()
    }

    fn run_compute<T, F: FnMut() -> T + 'c>(&self, compute: F, input_backwards_offsets: &mut Vec<usize>) -> T {
        debug_assert!(input_backwards_offsets.is_empty());

        let result = compute();
        let input_indices = self.post_read();
        let len = self.next_index();

        input_indices
            .into_iter()
            .map(|index| len - index + 1)
            .collect_into(input_backwards_offsets);
        (result, input_backwards_offsets)
    }

    fn post_read(&self) -> Vec<usize> {
        let mut results = Vec::new();
        for (index, current) in self.0.iter().enumerate() {
            if current.post_read() {
                results.push(index)
            }
        }
        results
    }

    /// Update all `Var`s with their new values and recompute `Rx`s.
    pub fn recompute(&mut self) {
        for (inputs, current, outputs) in self.0.as_mut().iter_mut_split3s() {
            current.recompute(inputs, outputs);
        }

        for current in self.0.as_mut().iter_mut() {
            current.post_recompute();
        }
    }
}

impl<'c> RxDAGElem<'c> {
    fn post_read(&self) -> bool {
        match self {
            RxDAGElem::Node(node) => node.post_read(),
            RxDAGElem::Edge(_) => {}
        }
    }

    fn recompute(&mut self, inputs: &[RxDAGElem], outputs: &[RxDAGElem]) {
        match self {
            RxDAGElem::Node(x) => x.recompute(),
            RxDAGElem::Edge(x) => x.recompute(inputs, outputs)
        }
    }

    fn post_recompute(&mut self) {
        match self {
            RxDAGElem::Node(x) => x.post_recompute(),
            RxDAGElem::Edge(_) => {}
        }
    }

    fn as_node(&self) -> Option<&Rx<'c>> {
        match self {
            RxDAGElem::Node(x) => Some(x),
            _ => None
        }
    }
}

impl<T> RxImpl<T> {
    fn new(init: T) -> Self {
        Self {
            current: init,
            next: Cell::new(None),
            did_read: Cell::new(false),
            did_recompute: false
        }
    }

    fn get(&self) -> &T {
        self.did_read.set(true);
        &self.current
    }

    fn set(&self, value: T) {
        self.next.set(Some(value));
    }
}

impl<'c, T> RxRef<'c, T> {
    fn new(graph: &RxDAG<'c>, index: usize) -> Self {
        RxRef {
            index,
            graph_id: graph.1,
            phantom: PhantomData
        }
    }

    fn get<'a>(&self, graph: &'a RxDAG<'c>) -> &'a T {
        self.get_rx(graph).get()
    }

    fn set(&self, graph: &RxDAG<'c>, value: T) {
        self.get_rx(graph).set(Some(value));
    }

    fn get_rx<'a>(&self, graph: &'a RxDAG<'c>) -> &'a RxImpl<T> {
        debug_assert!(self.graph_id == graph.0, "RxRef::get_rx: different graph");
        graph.0.get(self.index).expect("RxRef corrupt: index is an edge")
    }
}

impl<'c, T> Var<'c, T> {
    pub fn get<'a>(&self, c: &'a dyn RxContext<'c>) -> &'a T {
        let graph = c.graph();
        self.0.get(graph)
    }

    pub fn set(&mut self, c: &dyn RxContext<'c>, value: T) {
        let graph = c.graph();
        self.0.set(value, graph);
    }
}

impl<'c, T> CRx<'c, T> {
    pub fn get<'a>(&self, c: &'a dyn RxContext<'c>) -> &'a T {
        let graph = c.graph();
        self.0.get(graph)
    }
}

impl<T> RxTrait for RxImpl<T> {
    fn post_read(&self) -> bool {
        self.did_read.take()
    }

    fn recompute(&mut self) {
        debug_assert!(!self.did_recompute);
        match self.next.take() {
            // Didn't update
            None => {}
            // Did update
            Some(next) => {
                self.current = next;
                self.did_recompute = true;
            }
        }
    }

    fn did_recompute(&self) -> bool {
        self.did_recompute
    }

    fn post_recompute(&mut self) {
        self.did_recompute = false;
    }

    unsafe fn _set_dyn(&self, ptr: *mut u8, size: usize) {
        let mut value = MaybeUninit::<T>::uninit();
        ptr::copy_nonoverlapping(ptr, value.as_mut_ptr() as *mut u8, size);
        self.set(value.assume_init());
    }
}

impl<'c> Deref for RxDAGElem<'c> {
    type Target = Rx<'c>;

    fn deref(&self) -> &Self::Target {
        self.as_node().expect("RxRef is corrupt: index is an edge (cannot deref RxDAGElem which is an edge)")
    }
}

unsafe impl<'c> StableDeref for RxDAGElem<'c> {}

impl<'c, F: FnMut(&mut Vec<usize>, &mut dyn Iterator<Item=&Rx<'c>>) + 'c> RxEdgeImpl<'c, F> {
    fn new(input_backwards_offsets: Vec<usize>, num_outputs: usize, compute: F) -> Self {
        Self {
            input_backwards_offsets,
            num_outputs,
            compute
        }
    }

    fn output_forwards_offsets(&self) -> impl Iterator<Item=usize> {
        // Maybe this is a dumb abstraction.
        // This is very simple, outputs are currently always right after the edge.
        0..self.num_outputs
    }
}

impl<'c, F: FnMut(&mut Vec<usize>, &mut dyn Iterator<Item=&Rx<'c>>) + 'c> RxEdgeTrait for RxEdgeImpl<'c, F> {
    fn recompute(&mut self, inputs: &[RxDAGElem], outputs: &[RxDAGElem]) {
        let mut inputs = self.input_backwards_offsets.iter().map(|offset| {
            inputs[inputs.len() - offset].as_node().expect("broken RxDAG: RxEdge input must be a node")
        });

        if inputs.any(|x| x.did_recompute()) {
            // Needs update
            let mut outputs = self.output_forwards_offsets().map(|offset| {
                outputs[offset].as_node().expect("broken RxDAG: RxEdge output must be a node")
            });
            (self.compute)(&mut self.input_backwards_offsets, &mut outputs);
        }
    }
}

#[cfg(test)]
pub mod tests {
    use test_log::test;
    use super::*;
    use super::run_rx::run_rx;
    use super::snapshot_ctx::SNAPSHOT_CTX;

    #[test]
    fn test_srx() {
        let rx = SRx::new(vec![1, 2, 3]);
        assert_eq!(rx.get(SNAPSHOT_CTX).deref(), &vec![1, 2, 3]);
        rx.set(vec![1, 2, 4]);
        assert_eq!(rx.get(SNAPSHOT_CTX).deref(), &vec![1, 2, 4]);
        rx.set(vec![1, 2, 5]);
        assert_eq!(rx.get(SNAPSHOT_CTX).deref(), &vec![1, 2, 5]);
    }

    #[test]
    fn test_drx() {
        let mut rx = SRx::new(vec![1, 2, 3]);
        {
            let drx = rx.map_mut(|x| x.get_mut(0).unwrap());
            assert_eq!(drx.get(SNAPSHOT_CTX).deref(), &1);
            drx.set(2);
            assert_eq!(drx.get(SNAPSHOT_CTX).deref(), &2);
        }
        {
            let drx2 = rx.map_mut(|x| x.get_mut(2).unwrap());
            assert_eq!(drx2.get(SNAPSHOT_CTX).deref(), &3);
            drx2.modify(|x| *x += 2);
            assert_eq!(drx2.get(SNAPSHOT_CTX).deref(), &5);
        }
        assert_eq!(rx.get(SNAPSHOT_CTX).deref(), &vec![2, 2, 5]);
    }

    #[test]
    fn test_drx_split() {
        let mut rx = SRx::new(vec![1, 2, 3]);
        {
            let (drx0, drx1, drx2) = rx.split_map_mut3(|x| {
                let mut iter = x.iter_mut();
                (iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap())
            });
            assert_eq!(drx0.get(SNAPSHOT_CTX).deref(), &1);
            assert_eq!(drx1.get(SNAPSHOT_CTX).deref(), &2);
            assert_eq!(drx2.get(SNAPSHOT_CTX).deref(), &3);
            drx0.set(2);
            drx1.set(3);
            drx2.set(4);
        }
        assert_eq!(rx.get(SNAPSHOT_CTX).deref(), &vec![2, 3, 4]);
    }

    #[test]
    fn test_crx() {
        let rx = SRx::new(vec![1, 2, 3]);
        {
            let crx = CRx::new(|c| rx.get(c)[0] * 2);
            let crx2 = CRx::new(|c| *crx.get(c) + rx.get(c)[1] * 10);
            let crx3 = crx.map(|x| x.to_string());
            assert_eq!(*crx.get(SNAPSHOT_CTX), 2);
            assert_eq!(*crx2.get(SNAPSHOT_CTX), 22);
            assert_eq!(&*crx3.get(SNAPSHOT_CTX), "2");
            rx.set(vec![2, 3, 4]);
            assert_eq!(*crx.get(SNAPSHOT_CTX), 4);
            assert_eq!(*crx2.get(SNAPSHOT_CTX), 34);
            assert_eq!(&*crx3.get(SNAPSHOT_CTX), "4");
            rx.set(vec![3, 4, 5]);
            assert_eq!(*crx.get(SNAPSHOT_CTX), 6);
            assert_eq!(*crx2.get(SNAPSHOT_CTX), 46);
            assert_eq!(&*crx3.get(SNAPSHOT_CTX), "6");
        }
    }

    #[test]
    fn test_complex_rx_tree() {
        let mut rx1 = SRx::new(vec![1, 2, 3, 4]);
        {
            let (rx2_0, rx2_1, rx2_2, rx2_3) = rx1.split_map_mut4(|x| {
                let mut iter = x.iter_mut();
                (iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap())
            });
            let rx1_alt = CRx::new(|c| vec![*rx2_0.get(c), *rx2_1.get(c), *rx2_2.get(c), *rx2_3.get(c)]);
            let rx3 = CRx::new(|c| vec![*rx2_0.get(c) * 0, *rx2_1.get(c) * 1, *rx2_2.get(c) * 3, *rx2_3.get(c) * 4]);
            let rx4 = CRx::new(|c| rx3.get(c).iter().copied().zip(rx1_alt.get(c).iter().copied()).map(|(a, b)| a + b).collect::<Vec<_>>());
            let (_rx5_0, _rx5_1, _rx5_3) = rx4.split_map_ref3(|x| (&x[0], &x[1], &x[3]));
            assert_eq!(&*rx4.get(SNAPSHOT_CTX), &vec![1, 4, 9, 16, 25]);
            rx2_1.set(8);
            rx2_0.set(25);
            assert_eq!(&*rx4.get(SNAPSHOT_CTX), &vec![25, 16, 9, 16, 25]);
        }
        rx1.set(vec![5, 4, 3, 2, 1]);
        {
            let (rx2_0, rx2_1, rx2_2, rx2_3) = rx1.split_map_mut4(|x| {
                let mut iter = x.iter_mut();
                (iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap())
            });
            let rx1_alt = CRx::new(|c| vec![*rx2_0.get(c), *rx2_1.get(c), *rx2_2.get(c), *rx2_3.get(c)]);
            let rx3 = CRx::new(|c| vec![*rx2_0.get(c) * 0, *rx2_1.get(c) * 1, *rx2_2.get(c) * 3, *rx2_3.get(c) * 4]);
            let rx4 = CRx::new(|c| rx3.get(c).iter().copied().zip(rx1_alt.get(c).iter().copied()).map(|(a, b)| a + b).collect::<Vec<_>>());
            let (_rx5_0, _rx5_1, _rx5_3) = rx4.split_map_ref3(|x| (&x[0], &x[1], &x[3]));
            assert_eq!(&*rx4.get(SNAPSHOT_CTX), &vec![5, 8, 9, 8, 5]);
        }
    }

    #[test]
    fn test_run_rx() {
        let rx = SRx::new(1);
        let mut rx_snapshots = Vec::new();
        let mut expected_rx_snapshots = Vec::new();
        run_rx(|c| {
            rx_snapshots.push(*rx.get(c))
        });
        for i in 0..1000 {
            let new_value = *rx.get(SNAPSHOT_CTX) + 1;
            rx.set(new_value);
            expected_rx_snapshots.push(i + 1);
        }
        expected_rx_snapshots.push(1001);
        assert_eq!(rx_snapshots, expected_rx_snapshots);
    }
}
