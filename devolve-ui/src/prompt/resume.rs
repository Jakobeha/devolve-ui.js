use std::fmt::{Debug, Formatter};
use std::pin::Pin;
use std::future::Future;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::task::Waker;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PromptResumeState {
    /// Already resumed or just created
    Inactive,
    /// Waiting for [RawPromptResume::result] to be set
    Pending,
    /// [RawPromptResume::result] was set and we are ready to resume
    Ready
}

/// Data stored in the [PromptComponentContext1] which only has one active [PromptComponentResume]
/// at a time, so we don't have to reallocate. It's also untyped on the result type.
pub struct RawPromptResume {
    result: Vec<u8>,
    state: PromptResumeState,
    wakers: Vec<Waker>
}
#[derive(Debug)]
pub struct PromptResume<'a, R>(&'a mut RawPromptResume, PhantomData<R>);

/// Since `R` is just a phantom type and returned by functions,
/// and [RawPromptResume] is [Unpin], we can implement [Unpin] here as well.
impl<'a, R> Unpin for PromptResume<'a, R> {}

impl RawPromptResume {
    pub(super) fn new() -> Self {
        Self {
            result: Vec::new(),
            state: PromptResumeState::Inactive,
            wakers: Vec::new()
        }
    }

    fn resume<R>(&mut self, result: R) {
        assert!(self.state == PromptResumeState::Pending, "already resumed, please check before you resume twice");
        debug_assert!(self.result.is_empty());

        // Move result into the untyped result buffer
        unsafe {
            self.result.set_len(std::mem::size_of::<R>());
            std::ptr::copy_nonoverlapping(&result as *const R as *const u8, self.result.as_mut_ptr(), std::mem::size_of::<R>());
        }

        // Signal we are ready and wake up
        self.state = PromptResumeState::Ready;
        for waker in self.wakers.drain(..) {
            waker.wake()
        }
    }

    fn poll<R>(&mut self, ctx: &mut std::task::Context<'_>) -> std::task::Poll<R> {
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
                    std::ptr::copy_nonoverlapping(self.result.as_ptr(), result.as_mut_ptr() as *mut u8, std::mem::size_of::<R>());
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
    /// Wraps an untyped prompt-resume. You are responsible for ensuring that data resumed and polled is of the correct type.
    pub(super) unsafe fn new(raw: &'a mut RawPromptResume) -> Self {
        debug_assert!(raw.state == PromptResumeState::Inactive, "PromptResume created when another PromptResume is active (how?)");
        debug_assert!(raw.wakers.is_empty(), "PromptResume created when another resume still has wakers (how?)");
        debug_assert!(raw.result.is_empty(), "PromptResume created when result is not empty (how?)");

        Self(raw, PhantomData)
    }

    pub fn resume(&mut self, result: R) {
        self.0.resume(result);
    }
}

impl<'a, R> Future for PromptResume<'a, R> {
    type Output = R;

    fn poll(self: Pin<&mut Self>, ctx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        self.get_mut().0.poll(ctx)
    }
}

impl Debug for RawPromptResume {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RawPromptResume")
            .field("result", &self.result)
            .field("state", &self.state)
            .field("wakers.len()", &self.wakers.len())
            .finish()
    }
}