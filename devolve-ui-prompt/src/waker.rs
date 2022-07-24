use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

pub(super) struct PromptWaker;

impl PromptWaker {
    const VTABLE: RawWakerVTable = RawWakerVTable::new(
        |ptr| Self::new(ptr as *const Box<dyn Fn()>),
        |ptr| unsafe { Self::wake(ptr as *const Box<dyn Fn()>) },
        |ptr| unsafe { Self::wake(ptr as *const Box<dyn Fn()>) },
        |_ptr| {},
    );


    fn new(poll: *const Box<dyn Fn()>) -> RawWaker {
        RawWaker::new(poll as *const (), &Self::VTABLE)
    }

    pub(super) unsafe fn wake(poll: *const Box<dyn Fn()>) {
        let poll = &*poll;
        poll();
    }

    pub(super) unsafe fn poll<F: Future<Output=()>>(poll_again: *const Box<dyn Fn()>, future_cell: *const RefCell<Option<F>>) {
        let future_cell = &*future_cell;
        // If borrow fails = we are already waking the prompt
        // If None = the future was already ready by the time we reached here
        // (idk if either situation is actually allowed to happen, doesn't matter)
        if let Ok(mut future_ref) = future_cell.try_borrow_mut() {
            if let Some(future) = future_ref.as_mut() {
                let future = Pin::new_unchecked(future);
                let waker = Waker::from_raw(Self::new(poll_again));
                let mut context = Context::from_waker(&waker);

                // Ok, poll. This is the only line which actually does stuff
                let poll = future.poll(&mut context);

                match poll {
                    Poll::Ready(()) => {
                        // Set to None so we don't poll again or return on .await
                        *future_ref = None;
                    },
                    Poll::Pending => {},
                }
            }
        }
    }
}
