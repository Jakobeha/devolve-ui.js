use std::task::{Context, Poll};
use std::fmt::{Debug, Formatter};
use std::pin::Pin;
use std::future::Future;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::task::Waker;
use derive_more::Display;

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

/// Analogous to `resolve` in JavaScript promises.
/// For `reject`, make `R` a [Result] type, and then call [PromptResume::resume] with an `Err`.
#[derive(Debug)]
pub struct PromptResume<'a, R>(
    // None = always pending. Hopefully this compiles to a regular pointer.
    Option<&'a mut RawPromptResume>,
    PhantomData<R>
);

#[derive(Debug, Display)]
pub enum PromptResumeError {
    #[display(fmt = "already resumed")]
    AlreadyResumed,
    #[display(fmt = "always pending")]
    AlwaysPending,
}

pub type PromptResumeResult = Result<(), PromptResumeError>;

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

    fn resume<R>(&mut self, result: R) -> PromptResumeResult {
        if self.state != PromptResumeState::Pending {
            return Err(PromptResumeError::AlreadyResumed);
        }
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

        Ok(())
    }

    fn poll<R>(&mut self, ctx: &mut Context<'_>) -> Poll<R> {
        match self.state {
            PromptResumeState::Inactive => panic!("PromptResume polled a) before it setup or b) after it already resumed and returned its result"),
            PromptResumeState::Pending => {
                self.wakers.push(ctx.waker().clone());
                Poll::Pending
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
                Poll::Ready(result)
            },
        }
    }
}

impl<'a, R> PromptResume<'a, R> {
    /// Returns a [PromptResume] which is always pending.
    /// Whenever it is [resume](PromptResume::resume)d, it will return an error.
    pub(super) const fn pending() -> Self {
        Self(None, PhantomData)
    }

    /// Wraps an untyped prompt-resume. You are responsible for ensuring that data resumed and polled is of the correct type.
    pub(super) unsafe fn new(raw: &'a mut RawPromptResume) -> Self {
        debug_assert!(raw.state == PromptResumeState::Inactive, "PromptResume created when another PromptResume is active (how?)");
        debug_assert!(raw.wakers.is_empty(), "PromptResume created when another resume still has wakers (how?)");
        debug_assert!(raw.result.is_empty(), "PromptResume created when result is not empty (how?)");

        Self(Some(raw), PhantomData)
    }

    /// Causes the outer [yield_] to resume with the result the first time it is called.
    /// Afterwards, this will return [PromptResumeError::AlreadyResumed] and the result will just be
    /// dropped. You can handle the error or discard if you don't care.
    pub fn resume(&mut self, result: R) -> PromptResumeResult {
        match self.0.as_mut() {
            None => Err(PromptResumeError::AlwaysPending),
            Some(x) => x.resume(result)
        }
    }
}

impl<'a, R> Future for PromptResume<'a, R> {
    type Output = R;

    /// When you call [PromptResume::resume], this will return `Ready` with the result.
    fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.get_mut().0.as_mut() {
            None => Poll::Pending,
            Some(x) => x.poll(ctx)
        }
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