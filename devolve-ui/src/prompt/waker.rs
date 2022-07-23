use std::marker::PhantomData;
use std::future::Future;
use std::pin::Pin;
use std::ptr::addr_of;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use crate::core::component::node::VNode;
use crate::core::view::layout::parent_bounds::SubLayout;
use crate::core::view::view::{VViewData, VViewType};
use crate::prompt::PromptPinned;

pub(super) struct PromptWaker;

enum Dummy {}

enum DummyProps {}

enum DummyViewData {}

struct DummyFuture<Output>(Dummy, PhantomData<Output>);

struct DummyIterator<T>(Dummy, PhantomData<T>);

impl VViewData for DummyViewData {
    type Children<'a> = DummyIterator<&'a VNode<Self>>;
    type ChildrenMut<'a> = DummyIterator<&'a mut VNode<Self>>;

    fn typ(&self) -> VViewType {
        unreachable!()
    }

    fn children(&self) -> Option<(Self::Children<'_>, SubLayout)> {
        unreachable!()
    }

    fn children_mut(&mut self) -> Option<(Self::ChildrenMut<'_>, SubLayout)> {
        unreachable!()
    }
}

impl<T> Iterator for DummyIterator<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        unreachable!()
    }
}

impl<Output> Future for DummyFuture<Output> {
    type Output = Output;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        unreachable!()
    }
}

impl PromptWaker {
    const VTABLE: RawWakerVTable = RawWakerVTable::new(
        |ptr| Self::new(ptr),
        |ptr| Self::poll(ptr),
        |ptr| Self::poll(ptr),
        |_ptr| {},
    );


    fn new(ptr: *const ()) -> RawWaker {
        RawWaker::new(ptr, &Self::VTABLE)
    }

    pub(super) fn poll(ptr: *const ()) {
        let ptr_casted = ptr as *const PromptPinned<DummyProps, DummyViewData, DummyFuture<()>>;

        let future_poll_fn = unsafe { &*addr_of!((*ptr_casted).future_poll_fn) };
        let future = unsafe { &*addr_of!((*ptr_casted).future) };

        // If borrow fails = we are already waking the prompt
        // If None = the future was already ready by the time we reached here
        // (idk if either situation is actually allowed to happen, doesn't matter)
        if let Ok(mut future_ref) = future.try_borrow_mut() {
            if let Some(future) = future_ref.as_mut() {
                let future = unsafe { Pin::new_unchecked(future) };
                let waker = unsafe { Waker::from_raw(Self::new(ptr)) };
                let mut context = Context::from_waker(&waker);

                // Ok, poll. This is the only line which actually does stuff
                let poll = future_poll_fn(future, &mut context);

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
