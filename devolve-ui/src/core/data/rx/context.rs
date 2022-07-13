use std::alloc::{alloc, Layout};
use std::cell::{Cell, Ref, RefCell};
use std::hash::Hash;
use std::mem::{align_of, align_of_val, MaybeUninit, size_of, size_of_val};
use std::ptr;
use std::ptr::NonNull;
use std::rc::{Rc, Weak};
use smallvec::SmallVec;
use crate::core::misc::cell_vec::CellVec;
use crate::core::misc::slice_split3::SliceSplit3;

// Later Rxs *must*n depend on earlier Rxs.
pub struct RxDAG<'c> {
    current: Vec<RxDAGElem<'c>>,
    future: CellVec<RxDAGElem<'c>>,
}

enum RxDAGElem<'c> {
    Node(Box<Rx<'c>>),
    Edge(Box<RxEdge<'c>>)
}

type Rx<'c> = dyn RxTrait + 'c;
type RxEdge<'c> = dyn RxEdgeTrait<'c> + 'c;

pub trait RxTrait {
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

trait RxEdgeTrait<'c> {
    fn recompute(&mut self, inputs: &[RxDAGElem], outputs: &[RxDAGElem]);
}

struct RxEdgeImpl<'c, F: FnMut(&mut Vec<usize>, &mut dyn Iterator<Item=&Rx<'c>>) + 'c> {
    // Takes current of input values (first argument) and sets next of output values (second argument).
    compute: F,
    num_outputs: usize,
    input_backwards_offsets: Vec<usize>,
}

pub struct RxRef<'a, 'c, T> {
    graph: &'a RxDAG<'c>,
    index: usize,
    cached: &'a Cell<Option<NonNull<RxImpl<T>>>>,
}

pub struct Var<'a, 'c, T>(RxRef<'a, 'c, T>);
pub struct CRx<'a, 'c, T>(RxRef<'a, 'c, T>);

impl<'c> RxDAG<'c> {
    pub fn new() -> Self {
        Self {
            current: Vec::new(),
            future: CellVec::new()
        }
    }

    pub fn new_var<T>(&self, init: T) -> Var<'_, 'c, T> {
        let index = self.next_index();
        let mut rx = RxImpl::new(init);
        self.future.push(RxDAGElem::Node(Box::new(rx)));
        Var(RxRef::new(index))
    }

    pub fn new_crx<T, F: FnMut() -> T>(&self, mut compute: F) -> CRx<'_, 'c, T> {
        let mut input_backwards_offsets = Vec::new();
        let init = self.run_compute(&mut compute, &mut input_backwards_offsets);
        let compute_edge = RxEdgeImpl::new(input_backwards_offsets, 1, move |mut input_backwards_offsets, outputs| {
            input_backwards_offsets.clear();
            let output = self.run_compute(&mut compute, &mut input_backwards_offsets);
            unsafe { outputs.next().unwrap().set_dyn(output); }
            debug_assert!(outputs.next().is_none());
        });
        self.future.push(RxDAGElem::Edge(Box::new(compute_edge)));

        let index = self.next_index();
        let mut rx = RxImpl::new(init);
        self.future.push(RxDAGElem::Node(Box::new(rx)));
        CRx(RxRef::new(index))
    }

    pub fn new_crx2<T1, T2, F: FnMut() -> (T1, T2)>(&self, mut compute: F) -> (CRx<'_, 'c, T1>, CRx<'_, 'c, T2>) {
        let mut input_backwards_offsets = Vec::new();
        let (init1, init2) = self.run_compute(&mut compute, &mut input_backwards_offsets);
        let compute_edge = RxEdgeImpl::new(input_backwards_offsets, 2, move |mut input_backwards_offsets, outputs| {
            input_backwards_offsets.clear();
            let (output1, output2) = self.run_compute(&mut compute, &mut input_backwards_offsets);
            unsafe { outputs.next().unwrap().set_dyn(output1); }
            unsafe { outputs.next().unwrap().set_dyn(output2); }
            debug_assert!(outputs.next().is_none());
        });
        self.future.push(RxDAGElem::Edge(Box::new(compute_edge)));

        let index = self.next_index();
        let mut rx1 = RxImpl::new(init1);
        let mut rx2 = RxImpl::new(init2);
        self.future.push(RxDAGElem::Node(Box::new(rx1)));
        self.future.push(RxDAGElem::Node(Box::new(rx2)));
        (CRx(RxRef::new(index)), CRx(RxRef::new(index + 1)))
    }

    pub fn new_crx3<T1, T2, T3, F: FnMut() -> (T1, T2, T3)>(&self, mut compute: F) -> (CRx<'_, 'c, T1>, CRx<'_, 'c, T2>, CRx<'_, 'c, T3>) {
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
        self.future.push(RxDAGElem::Edge(Box::new(compute_edge)));

        let index = self.next_index();
        let mut rx1 = RxImpl::new(init1);
        let mut rx2 = RxImpl::new(init2);
        let mut rx3 = RxImpl::new(init3);
        self.future.push(RxDAGElem::Node(Box::new(rx1)));
        self.future.push(RxDAGElem::Node(Box::new(rx2)));
        self.future.push(RxDAGElem::Node(Box::new(rx3)));
        (CRx(RxRef::new(index)), CRx(RxRef::new(index + 1)), CRx(RxRef::new(index + 2)))
    }

    fn next_index(&self) -> usize {
        self.current.len() + self.future.len()
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
        for (index, current) in self.current.iter().enumerate() {
            if current.post_read() {
                results.push(index)
            }
        }
        results
    }

    pub fn recompute(&mut self) {
        self.future.be_appended_to(&mut self.current);

        for (inputs, current, outputs) in self.current.iter_mut_split3s() {
            current.recompute(inputs, outputs);
        }

        for current in self.current.iter_mut() {
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

impl<'a, T> Var<'a, T> {
    pub fn get(&self) -> &T {
        self.0.get()
    }

    pub fn set(&mut self, value: T) {
        self.0.set(value);
    }
}

impl<'a, T> CRx<'a, T> {
    pub fn get(&self) -> &T {
        self.0.get()
    }
}

impl<T> RxTrait for RxImpl<T> {
    fn post_read(&self) -> bool {
        self.did_read.take()
    }

    fn did_recompute(&self) -> bool {
        self.did_recompute
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

    fn post_recompute(&mut self) {
        self.did_recompute = false;
    }

    unsafe fn _set_dyn(&self, ptr: *mut u8, size: usize) {
        let mut value = MaybeUninit::<T>::uninit();
        ptr::copy_nonoverlapping(ptr, value.as_mut_ptr() as *mut u8, size);
        self.set(value.assume_init());
    }
}

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

impl<'c, F: FnMut(&mut Vec<usize>, &mut dyn Iterator<Item=&Rx<'c>>) + 'c> RxEdgeTrait<'c> for RxEdgeImpl<'c, F> {
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