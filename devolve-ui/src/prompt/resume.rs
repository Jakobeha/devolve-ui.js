use std::any::Any;
use std::cell::{Cell, RefCell};
use std::pin::Pin;
use std::future::Future;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::task::Waker;
use crate::core::view::view::VViewData;
use crate::core::component::node::VNode;
use crate::core::component::context::VComponentContext1;

enum PromptResumeState {
    /// Already resumed or just created
    Inactive,
    /// Waiting for [RawPromptResume::result] to be set
    Pending,
    /// [RawPromptResume::result] was set and we are ready to resume
    Ready
}

/// Data stored in the [PromptComponentContext1] which only has one active [PromptComponentResume] at a time,
/// so we don't have to reallocate.
pub(super) struct RawPromptResume {
    result: Vec<u8>,
    state: PromptResumeState,
    wakers: Vec<Waker>
}

pub struct PromptResume<'a, R>(&'a mut RawPromptResume, PhantomData<R>);

impl RawPromptResume {
    pub(super) fn new() -> Self {
        Self {
            result: Vec::new(),
            state: PromptResumeState::Inactive,
            wakers: Vec::new()
        }
    }

    fn resume<R>(&mut self, result: R) {
        debug_assert!(self.state == PromptResumeState::Pending);
        debug_assert!(self.result.is_empty());

        // Move result into the untyped result buffer
        unsafe {
            self.result.set_len(std::mem::size_of::<R>());
            std::ptr::copy_nonoverlapping(&result, self.result.as_mut_ptr(), std::mem::size_of::<R>());
        }

        // Signal we are ready and wake up
        self.state = PromptResumeState::Ready;
        for waker in self.wakers.drain(..) {
            waker.wake()
        }
    }

    fn poll<R>(self: Pin<&mut Self>, ctx: &mut std::task::Context<'_>) -> std::task::Poll<R> {
        match self.state {
            PromptResumeState::Inactive => panic!("PromptResume polled a) before it setup or b) after it already resumed and returned its result"),
            PromptResumeState::Pending => {
                self.wakers.push(ctx.waker().clone());
                std::task::Poll::Pending
            }
            PromptResumeState::Ready => {
                // Signal we are inactive for future polls
                self.state = PromptResumeState::Inactive;

                // Move result out of the untyped result buffer
                let result = unsafe {
                    let mut result = MaybeUninit::<R>::uninit();
                    std::ptr::copy_nonoverlapping(self.result.as_ptr(), result.as_mut_ptr(), std::mem::size_of::<R>());
                    self.result.clear();
                    result.assume_init()
                };

                // Return that we are ready and have the result
                std::task::Poll::Ready(result)
            }
        }

    }
}

impl<'a, R> PromptResume<'a, R> {
    pub(super) fn new(raw: &'a mut RawPromptResume) -> Self {
        debug_assert!(raw.state == PromptResumeState::Inactive, "PromptResume created when another PromptResume is active (how?)");
        debug_assert!(raw.wakers.is_empty(), "PromptResume created when another resume still has wakers (how?)");
        debug_assert!(raw.result.is_empty(), "PromptResume created when result is not empty (how?)");

        Self(raw, PhantomData)
    }

    pub fn resume(&mut self, result: R) {
        self.0.resume(result);
    }
}

impl<R> Future for PromptResume<R> {
    type Output = R;

    fn poll(self: Pin<&mut Self>, ctx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        self.0.poll(ctx)
    }
}