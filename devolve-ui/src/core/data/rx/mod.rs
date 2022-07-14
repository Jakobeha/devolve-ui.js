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
use std::fmt::{Debug, Formatter};
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

/// Returns a graph you can write. Note that `RxContext` and `MutRxContext` are neither subset nor superset of each other.
/// You can't read snapshots without recomputing, and you can't write inputs.
pub trait RxContext<'a, 'c> {
    fn sub_dag(self) -> RxSubDAG<'a, 'c>;
}

/// Returns a graph you can write. Note that `RxContext` and `MutRxContext` are neither subset nor superset of each other.
/// You can't read snapshots without recomputing, and you can't write inputs.
pub trait MutRxContext<'a, 'c> {
    fn sub_dag(self) -> RxSubDAG<'a, 'c>;
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
#[derive(Debug)]
pub struct RxDAG<'c>(FrozenVec<RxDAGElem<'c>>, RxDAGUid<'c>);

#[derive(Debug, Clone, Copy)]
pub struct RxDAGSnapshot<'a, 'c>(&'a RxDAG<'c>);

#[derive(Debug, Clone, Copy)]
pub struct RxSubDAG<'a, 'c> {
    before: &'a [RxDAGElem<'c>],
    index: usize,
    id: RxDAGUid<'c>
}

#[derive(Debug, Clone, Copy)]
pub struct RxInput<'a, 'c>(RxSubDAG<'a, 'c>);

enum RxDAGElem<'c> {
    Node(Box<Rx<'c>>),
    Edge(Box<RxEdge<'c>>)
}

type Rx<'c> = dyn RxTrait + 'c;
type RxEdge<'c> = dyn RxEdgeTrait + 'c;

trait RxTrait: Debug {
    fn post_read(&self) -> bool;

    fn recompute(&mut self);
    fn did_recompute(&self) -> bool;
    fn post_recompute(&mut self);

    unsafe fn _set_dyn(&self, ptr: *mut u8, size: usize);
}

struct RxImpl<T> {
    current: T,
    next: Cell<Option<T>>,
    // Rx flags (might have same flags for a group to reduce traversing all Rxs)
    did_read: Cell<bool>,
    did_recompute: bool
}

trait RxEdgeTrait: Debug {
    fn recompute(&mut self, index: usize, before: &[RxDAGElem], after: &[RxDAGElem], graph_id: RxDAGUid);
}

struct RxEdgeImpl<'c, F: FnMut(&mut Vec<usize>, RxSubDAG<'_, 'c>, &mut dyn Iterator<Item=&Rx<'c>>) + 'c> {
    // Takes current of input values (first argument) and sets next of output values (second argument).
    compute: F,
    num_outputs: usize,
    input_backwards_offsets: Vec<usize>,
    cached_inputs: Vec<*const RxDAGElem<'c>>
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

/// View and mutate part of a `Var`
#[derive(Debug)]
pub struct DVar<'c, S, T, GetFn: Fn(&S) -> &T, SetFn: Fn(&S, T) -> S> {
    source: RxRef<'c, S>,
    get: GetFn,
    set: SetFn
}

#[derive(Debug)]
pub struct DCRx<'c, S, T, GetFn: Fn(&S) -> &T> {
    source: RxRef<'c, S>,
    get: GetFn
}

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

    /// Run a closure when inputs change, without creating any outputs (for side-effects).
    pub fn run_crx<F: FnMut(RxInput<'_, 'c>)>(&self, mut compute: F) {
        let mut input_backwards_offsets = Vec::new();
        let () = self.run_compute(&mut compute, RxInput(self.sub_dag()), &mut input_backwards_offsets);
        let compute_edge = RxEdgeImpl::new(input_backwards_offsets, 0, move |mut input_backwards_offsets, input, outputs| {
            input_backwards_offsets.clear();
            let () = self.run_compute(&mut compute, input, &mut input_backwards_offsets);
            debug_assert!(outputs.next().is_none());
        });
        self.0.push(RxDAGElem::Edge(Box::new(compute_edge)));
    }

    /// Create a computed `Rx` in this DAG.
    pub fn new_crx<T, F: FnMut(RxInput<'_, 'c>) -> T>(&self, mut compute: F) -> CRx<'c, T> {
        let mut input_backwards_offsets = Vec::new();
        let init = self.run_compute(&mut compute, RxInput(self.sub_dag()), &mut input_backwards_offsets);
        let compute_edge = RxEdgeImpl::new(input_backwards_offsets, 1, move |mut input_backwards_offsets, input, outputs| {
            input_backwards_offsets.clear();
            let output = self.run_compute(&mut compute, input, &mut input_backwards_offsets);
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
    pub fn new_crx2<T1, T2, F: FnMut(RxInput<'_, 'c>) -> (T1, T2)>(&self, mut compute: F) -> (CRx<'c, T1>, CRx<'c, T2>) {
        let mut input_backwards_offsets = Vec::new();
        let (init1, init2) = self.run_compute(&mut compute, RxInput(Self.sub_dag()), &mut input_backwards_offsets);
        let compute_edge = RxEdgeImpl::new(input_backwards_offsets, 2, move |mut input_backwards_offsets, input, outputs| {
            input_backwards_offsets.clear();
            let (output1, output2) = self.run_compute(&mut compute, input, &mut input_backwards_offsets);
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
    pub fn new_crx3<T1, T2, T3, F: FnMut(RxInput<'_, 'c>) -> (T1, T2, T3)>(&self, mut compute: F) -> (CRx<'c, T1>, CRx<'c, T2>, CRx<'c, T3>) {
        let mut input_backwards_offsets = Vec::new();
        let (init1, init2, init3) = self.run_compute(&mut compute, RxInput(self.sub_dag), &mut input_backwards_offsets);
        let compute_edge = RxEdgeImpl::new(input_backwards_offsets, 2, move |mut input_backwards_offsets, input, outputs| {
            input_backwards_offsets.clear();
            let (output1, output2, output3) = self.run_compute(&mut compute, input, &mut input_backwards_offsets);
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

    fn run_compute<T, F: FnMut(RxInput<'_, 'c>) -> T + 'c>(&self, compute: F, input: RxInput<'_, 'c>, input_backwards_offsets: &mut Vec<usize>) -> T {
        debug_assert!(input_backwards_offsets.is_empty());

        let result = compute(input);
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
        for (index, (before, current, after)) in self.0.as_mut().iter_mut_split3s().enumerate() {
            current.recompute(index, before, after, self.1);
        }

        for current in self.0.as_mut().iter_mut() {
            current.post_recompute();
        }
    }

    /// Recomputes if necessary and then returns a `RxContext` you can use to get the current value.
    pub fn now(&mut self) -> RxDAGSnapshot<'_, 'c> {
        self.recompute();
        RxDAGSnapshot(self)
    }
}

impl<'a, 'c> RxContext<'a, 'c> for RxDAGSnapshot<'a, 'c> {
    fn sub_dag(self) -> RxSubDAG<'a, 'c> {
        RxSubDAG {
            before: unsafe { std::mem::transmute(&self.0) },
            index: self.0.len(),
            id: self.1
        }
    }
}

impl<'a, 'c> MutRxContext<'a, 'c> for &'a RxDAG<'c> {
    fn sub_dag(self) -> RxSubDAG<'a, 'c> {
        RxDAGSnapshot(self).sub_dag()
    }
}

impl<'a, 'c> RxSubDAG<'a, 'c> {
    fn get(&self, index: usize) -> &RxDAGElem<'a> {
        debug_assert!(index < self.index, "index out of sub-dag bounds");
        &self.before[index]
    }
}

impl<'a, 'c> RxContext<'c> for RxInput<'a, 'c> {
    fn sub_dag(self) -> RxSubDAG<'a, 'c> {
        self.0
    }
}

impl<'c> RxDAGElem<'c> {
    fn post_read(&self) -> bool {
        match self {
            RxDAGElem::Node(node) => node.post_read(),
            RxDAGElem::Edge(_) => {}
        }
    }

    fn recompute(&mut self, index: usize, before: &[RxDAGElem], after: &[RxDAGElem], graph_id: RxDAGUid) {
        match self {
            RxDAGElem::Node(x) => x.recompute(),
            RxDAGElem::Edge(x) => x.recompute(index, before, after, graph_id)
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

    fn get<'a>(&self, graph: RxSubDAG<'a, 'c>) -> &'a T {
        self.get_rx(graph).get()
    }

    fn set(&self, graph: RxSubDAG<'_, 'c>, value: T) {
        self.get_rx(graph).set(Some(value));
    }

    fn get_rx<'a>(&self, graph: RxSubDAG<'a, 'c>) -> &'a RxImpl<T> {
        debug_assert!(self.graph_id == graph.0, "RxRef::get_rx: different graph");
        graph.get(self.index).expect("RxRef corrupt: index is an edge")
    }
}

impl<'c, T> Var<'c, T> {
    pub fn get<'a>(&self, c: impl RxContext<'a, 'c>) -> &'a T {
        let graph = c.sub_dag();
        self.0.get(graph)
    }

    pub fn set(&self, c: impl MutRxContext<'_, 'c>, value: T) {
        let graph = c.sub_dag();
        self.0.set(value, graph);
    }

    pub fn derive<U, GetFn: Fn(&T) -> &U, SetFn: Fn(&T, U) -> T>(&self, get: GetFn, set: SetFn) -> DVar<T, U, GetFn, SetFn> {
        DVar {
            source: &self.0 as RxRef<T>,
            get,
            set
        }
    }

    pub fn derive_using_clone<U, GetFn: Fn(&T) -> &U, SetFn: Fn(&mut T, U)>(&self, get: GetFn, set: SetFn) -> DVar<T, U, GetFn, SetFn> where T: Clone {
        self.derive(get, move |root, child| {
            let mut root = root.clone();
            set(&mut root, child);
            root
        })
    }
}

impl<'c, T> CRx<'c, T> {
    pub fn get<'a>(&self, c: impl RxContext<'a, 'c>) -> &'a T {
        let graph = c.sub_dag();
        self.0.get(graph)
    }

    pub fn derive<U, GetFn: Fn(&T) -> &U>(&self, get: GetFn) -> DCRx<T, U, GetFn> {
        DCRx {
            source: &self.0 as RxRef<T>,
            get
        }
    }
}

impl<'c, S, T, GetFn: Fn(&S) -> &T, SetFn: Fn(&S, T) -> S> DVar<'c, S, T, GetFn, SetFn> {
    pub fn get<'a>(&self, c: impl RxContext<'a, 'c>) -> &'a T {
        let graph = c.sub_dag();
        (self.get)(self.source.get(graph))
    }

    pub fn set(&self, c: impl MutRxContext<'_, 'c>, value: T) {
        let graph = c.sub_dag();
        let old_value = self.source.get(graph);
        let new_value = (self.set)(old_value, &value);
        self.source.set(new_value, graph)
    }
}

impl<'c, S, T, GetFn: Fn(&S) -> &T> DCRx<'c, S, T, GetFn> {
    pub fn get<'a>(&self, c: impl RxContext<'a, 'c>) -> &'a T {
        let graph = c.sub_dag();
        (self.get)(self.source.get(graph))
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

impl<'c, F: FnMut(&mut Vec<usize>, RxInput<'_, 'c>, &mut dyn Iterator<Item=&Rx<'c>>) + 'c> RxEdgeImpl<'c, F> {
    fn new(input_backwards_offsets: Vec<usize>, num_outputs: usize, compute: F) -> Self {
        let num_inputs = input_backwards_offsets.len();
        Self {
            input_backwards_offsets,
            num_outputs,
            compute,
            cached_inputs: Vec::with_capacity(num_inputs)
        }
    }

    fn output_forwards_offsets(&self) -> impl Iterator<Item=usize> {
        // Maybe this is a dumb abstraction.
        // This is very simple, outputs are currently always right after the edge.
        0..self.num_outputs
    }
}

impl<'c, F: FnMut(&mut Vec<usize>, RxInput<'_, 'c>, &mut dyn Iterator<Item=&Rx<'c>>) + 'c> RxEdgeTrait for RxEdgeImpl<'c, F> {
    fn recompute(&mut self, index: usize, before: &[RxDAGElem], after: &[RxDAGElem], graph_id: RxDAGUid) {
        debug_assert!(self.cached_inputs.is_empty());
        self.input_backwards_offsets.iter().copied().map(|offset| {
            before[before.len() - offset].as_node().expect("broken RxDAG: RxEdge input must be a node") as *const _
        }).collect_into(&mut self.cached_inputs);
        let mut inputs = self.cached_inputs.iter().map(|x| unsafe { &**x });

        if inputs.any(|x| x.did_recompute()) {
            // Needs update
            let mut outputs = self.output_forwards_offsets().map(|offset| {
                after[offset].as_node().expect("broken RxDAG: RxEdge output must be a node")
            });
            let input_dag = RxInput(RxSubDAG {
                before,
                index,
                id: graph_id
            });
            (self.compute)(&mut self.input_backwards_offsets, input_dag, &mut outputs);
        }
        self.cached_inputs.clear();
    }
}

impl dyn RxTrait {
    unsafe fn set_dyn<T>(&self, mut value: T) {
        self._set_dyn(&mut value as *mut T as *mut u8, size_of_val(&value));
    }
}

impl<T> Debug for RxImpl<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RxImpl")
            .field("next.is_some()", &unsafe { &*self.next.as_ptr() }.is_some())
            .field("did_read", &self.did_read.get())
            .field("did_recompute", &self.did_recompute)
            .finish_non_exhaustive()
    }
}

impl<T> Debug for RxEdgeImpl<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RxEdgeImpl")
            .field("num_outputs", &self.num_outputs)
            .field("input_backwards_offsets", &self.input_backwards_offsets)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
pub mod tests {
    use test_log::test;
    use super::*;
    use super::run_rx::run_rx;
    use super::snapshot_ctx::SNAPSHOT_CTX;

    #[test]
    fn test_rx() {
        let mut g = RxDAG::new();
        let mut rx = g.new_var(1);
        let mut crx = g.new_rx(|g| *rx.get(g) * 2);
        let mut side_effect = 1;
        g.run_crx(|g| {
            side_effect += crx.get(g);
        });
        assert_eq!(rx.get(g.now()), &1);
        assert_eq!(crx.get(g.now()), &2);
        assert_eq!(side_effect, 2);

        rx.set(&g, 2);
        assert_eq!(rx.get(g.now()), &2);
        assert_eq!(crx.get(g.now()), &4);
        assert_eq!(side_effect, 6);

        rx.set(&g, 3);
        assert_eq!(rx.get(g.now()), &3);
        assert_eq!(crx.get(g.now()), &6);
        assert_eq!(side_effect, 12);
    }

    #[test]
    fn test_rx_multiple_inputs_outputs() {
        let mut g = RxDAG::new();
        let mut rx = g.new_var(1);
        let mut rx2 = g.new_var(2);
        let mut rx3 = g.new_var(vec![3, 4]);
        {
            let crx = g.new_crx(|g| vec![*rx.get(g) * 10, *rx2.get(g) * 10]);
            let crx2 = g.new_crx(|g| {
                let mut vec = Vec::new();
                vec.push(*rx.get(g));
                for elem in rx3.get(g).iter().copied() {
                    vec.push(elem)
                }
                for elem in crx.get(g).iter().copied() {
                    vec.push(elem)
                }
                vec
            });
            let (crx3_1, crx3_2) = g.new_crx2(|g| {
                let vec = crx.get(g);
                (vec[0] * 10, vec[1] * 10)
            });
            let (crx4_1, crx4_2, crx4_3) = g.new_crx3(|g| {
                let v2 = rx2.get(g);
                let v3 = crx3_1.get(g);
                let v4 = rx3.get(g)[0];
                (v2, v3, v4 * 100)
            });

            assert_eq!(rx.get(g.now()), &1);
            assert_eq!(rx2.get(g.now()), &2);
            assert_eq!(rx3.get(g.now()), &vec![3, 4]);
            assert_eq!(crx.get(g.now()), &vec![10, 20]);
            assert_eq!(crx2.get(g.now()), &vec![1, 3, 4, 10, 20]);
            assert_eq!(crx3_1.get(g.now()), &100);
            assert_eq!(crx3_2.get(g.now()), &200);
            assert_eq!(crx4_1.get(g.now()), &1);
            assert_eq!(crx4_2.get(g.now()), &200);
            assert_eq!(crx4_3.get(g.now()), &300);

            rx.set(&g, 5);
            rx2.set(&g, 6);
            rx3.set(&g, vec![7, 8, 9]);
            g.recompute();

            assert_eq!(rx.get(g.now()), &5);
            assert_eq!(rx2.get(g.now()), &6);
            assert_eq!(rx3.get(g.now()), &vec![7, 8, 9]);
            assert_eq!(crx.get(g.now()), &vec![50, 60]);
            assert_eq!(crx2.get(g.now()), &vec![5, 7, 8, 50, 60]);
            assert_eq!(crx3_1.get(g.now()), &500);
            assert_eq!(crx3_2.get(g.now()), &600);
            assert_eq!(crx4_1.get(g.now()), &5);
            assert_eq!(crx4_2.get(g.now()), &600);
            assert_eq!(crx4_3.get(g.now()), &700);
        }
    }

    #[test]
    fn test_drx_split() {
        let mut g = RxDAG::new();
        let rx = g.new_var(vec![1, 2, 3]);
        {
            let drx0 = rx.derive_using_clone(|x| &x[0], |x, new| {
                x[0] = new;
            });
            let drx1 = rx.derive_using_clone(|x| &x[1], |x, new| {
                x[1] = new;
            });
            let drx2 = rx.derive_using_clone(|x| &x[2], |x, new| {
                x[2] = new;
            });
            assert_eq!(drx0.get(g.now()).deref(), &1);
            assert_eq!(drx1.get(g.now()).deref(), &2);
            assert_eq!(drx2.get(g.now()).deref(), &3);
            drx0.set(&g, 2);
            drx1.set(&g, 3);
            drx2.set(&g, 4);
        }
        assert_eq!(rx.get(g.now()).deref(), &vec![2, 3, 4]);
    }

    #[test]
    fn test_crx() {
        let mut g = RxDAG::new();
        let rx = g.new_var(vec![1, 2, 3]);
        {
            let crx = g.new_crx(|g| rx.get(g)[0] * 2);
            let crx2 = g.new_crx(|g| *crx.get(g) + rx.get(g)[1] * 10);
            let crx3 = g.new_crx(|g| *crx2.get(g).to_string());
            assert_eq!(*crx.get(g.now()), 2);
            assert_eq!(*crx2.get(g.now()), 22);
            assert_eq!(&*crx3.get(g.now()), "2");
            rx.set(&g, vec![2, 3, 4]);
            assert_eq!(*crx.get(g.now()), 4);
            assert_eq!(*crx2.get(g.now()), 34);
            assert_eq!(&*crx3.get(g.now()), "4");
            rx.set(&g, vec![3, 4, 5]);
            assert_eq!(*crx.get(g.now()), 6);
            assert_eq!(*crx2.get(g.now()), 46);
            assert_eq!(&*crx3.get(g.now()), "6");
        }
    }

    /* #[test]
    fn test_complex_rx_tree() {
        todo!()
    } */
}