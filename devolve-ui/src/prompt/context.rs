//! Prompt contexts.
//! These are different than regular component [VComponentContext],
//! because regular components render instantaneously but prompt-component functions run over a lifetime.

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::future::{Future, IntoFuture, Ready, ready};
use std::marker::PhantomData;
use std::pin::Pin;
use std::ptr::{addr_of, addr_of_mut, null_mut};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use crate::core::component::context::{VComponentContext1, VComponentContext2};
use crate::core::component::node::VNode;
use crate::core::view::view::{VViewData, VViewType};
use crate::core::misc::either_future::EitherFuture;
use crate::core::view::layout::parent_bounds::SubLayout;
use crate::prompt::resume::{PromptResume, RawPromptResume};

// region type declarations
pub struct VPrompt<
    Props: Any,
    ViewData: VViewData,
    F: Future<Output=()>
>(Pin<Box<PromptPinned<Props, ViewData, F>>>);

struct PromptPinned<
    Props: Any,
    ViewData: VViewData,
    F: Future<Output=()>
> {
    future_poll_fn: fn(Pin<&mut F>, &mut Context<'_>) -> Poll<()>,
    future: RefCell<Option<F>>,
    context_data: PromptContextData<Props, ViewData>
}

struct PromptContextData<
    Props: Any,
    ViewData: VViewData
> {
    current: Option<Box<dyn FnMut(VRawPromptComponentContext<'_, '_, Props, ViewData>) -> VNode<ViewData>>>,
    resume: RawPromptResume,
    phantom: PhantomData<Props>
}

/// Context within a prompt-function. This provies [yield_], which allows you to actually render prompts.
///
/// **Note:** this type may exist longer than the actual [VPrompt] it was created from. Once the [VPrompt] is destroyed,
/// the context will simply block forever the next time you try to yield anything.
pub struct VPromptContext<Props: Any, ViewData: VViewData>(*mut PromptContextData<Props, ViewData>);

type VRawPromptComponentContext<'a, 'a0, Props, ViewData> = (VComponentContext1<'a, 'a0, Props, ViewData>, &'a mut RawPromptResume, &'a Props);
pub type VPromptComponentContext<'a, 'a0, Props, ViewData, R> = (VComponentContext1<'a, 'a0, Props, ViewData>, PromptResume<'a, R>, &'a Props);
pub type VPromptContext2<Props, ViewData, PromptProps> = (VPromptContext<Props, ViewData>, PromptProps);

struct PromptWaker;

#[derive(Debug, PartialEq, Eq)]
enum WhichPartOfThePromptContextDiedFirst {
    ContextData,
    ContextPtrWrapper
}
// endregion

impl<
    Props: Any,
    ViewData: VViewData + 'static,
    F: Future<Output=()> + 'static
> VPrompt<Props, ViewData, F> {
    pub fn new<PromptProps>(prompt_fn: impl FnOnce(VPromptContext2<Props, ViewData, PromptProps>) -> F, prompt_props: PromptProps) -> Self {
        // Setup uninit addresses
        let mut pinned = Box::<PromptPinned<Props, ViewData, F>>::new_uninit();
        let future_poll_fn = unsafe { addr_of_mut!((*pinned.as_mut_ptr()).future_poll_fn) };
        let future = unsafe { addr_of_mut!((*pinned.as_mut_ptr()).future) };
        let context_data = unsafe { addr_of_mut!((*pinned.as_mut_ptr()).context_data) };

        // (future poll fn is statically known)
        unsafe { future_poll_fn.write(F::poll) };

        // Setup context data
        let the_context_data = PromptContextData {
            current: None,
            resume: RawPromptResume::new(),
            phantom: PhantomData
        };
        unsafe { context_data.write(the_context_data); }
        let context = VPromptContext::new(context_data);

        // Get future with pinned setup context data data
        let the_future = prompt_fn((context, prompt_props));
        unsafe { future.write(RefCell::new(Some(the_future))) };

        // Poll the future once, PromptWaker will take care of future polling
        PromptWaker::poll(pinned.as_ptr() as *const ());

        // Check that we set current to something before await
        assert!(unsafe { &*context_data }.current.is_some(), "prompt functions must yield something before awaiting. Yield a \"loading\" or empty component if you're not ready");

        // Ok we are ready
        Self(unsafe { Pin::new_unchecked(pinned.assume_init()) })
    }
}

impl<
    Props: Any,
    ViewData: VViewData,
    F: Future<Output=()>
> VPrompt<Props, ViewData, F> {
    pub fn current(&mut self, (c, props): VComponentContext2<'_, '_, Props, ViewData>) -> VNode<ViewData> {
        // SAFETY: we aren't moving any of the data in this, except possibly context_data.resume, but that is Unpin
        let pinned = unsafe { self.0.as_mut().get_unchecked_mut() };
        let current = pinned.context_data.current.as_mut().expect("prompt is still being created, you can't get current component yet");
        current((c, assert_is_unpin(&mut pinned.context_data.resume), props))
    }
}

impl<
    Props: Any,
    ViewData: VViewData,
    F: Future<Output=()>
> IntoFuture for VPrompt<Props, ViewData, F> {
    type Output = ();
    type IntoFuture = EitherFuture<Ready<()>, F, ()>;

    /// Returns a future which will complete when the wrapped prompt function does.
    fn into_future(self) -> Self::IntoFuture {
        match self.0.future.take() {
            None => EitherFuture::Left(ready(())),
            Some(future) => EitherFuture::Right(future)
        }
    }
}

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

    pub fn poll(ptr: *const ()) {
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

// region PromptContextData and VPromptContext
impl<Props: Any, ViewData: VViewData> PromptContextData<Props, ViewData> {
    pub fn yield_<'a, R>(
        self: Pin<&'a mut Self>,
        mut render: impl FnMut(VPromptComponentContext<'_, '_, Props, ViewData, R>) -> VNode<ViewData> + 'static
    ) -> PromptResume<'a, R> {
        unsafe {
            let resume = self.yield_raw(Box::new(move |(c, resume, props)| {
                let resume = PromptResume::new(resume);
                render((c, resume, props))
            }));
            PromptResume::new(resume)
        }
    }

    pub fn yield_void<'a>(
        self: Pin<&'a mut Self>,
        render: impl FnMut(VPromptComponentContext<'_, '_, Props, ViewData, ()>) -> VNode<ViewData> + 'static
    ) -> PromptResume<'a, ()> {
        self.yield_(render)
    }

    unsafe fn yield_raw<'a>(
        self: Pin<&'a mut Self>,
        render: Box<dyn FnMut(VRawPromptComponentContext<'_, '_, Props, ViewData>) -> VNode<ViewData>>
    ) -> &'a mut RawPromptResume {
        // SAFETY: We only set and return a value which implements Unpin
        let this = self.get_unchecked_mut();
        this.current = Some(render);
        // TODO: If in a regular component, signal to the renderer that we need to update
        assert_is_unpin(&mut this.resume)
    }
}

// Absolutely nothing here is thread-safe
impl<Props: Any, ViewData: VViewData> !Sync for PromptContextData<Props, ViewData> {}
impl<Props: Any, ViewData: VViewData> !Send for PromptContextData<Props, ViewData> {}

impl<Props: Any, ViewData: VViewData> VPromptContext<Props, ViewData> {
    fn new(context_data: *mut PromptContextData<Props, ViewData>) -> Self {
        VPromptContext(context_data)
    }

    /// Makes the prompt function call [render] whenever it needs to render as a component,
    /// from when this is called until the next [yield_] is called.
    ///
    /// Also blocks until [resume](PromptResume::resume) is called from the provided [PromptResume].
    ///
    /// Note that even after [PromptResume::resume] is called, [render] will still be subsequently
    /// called when the prompt needs to re-render. You can only guarantee [render] will no longer be
    /// called once the next [yield_] is called.
    ///
    /// This can be useful in some cases: e.g. if you have an image you need to load, you will call
    /// [yield_] with a function which renders "loading"  and calls [PromptResume::resume]
    /// immediately. Then you can `await` loading the image and once done, call [yield_] to
    /// render the result.
    ///
    /// Also note that if the prompt no longer actually exists, this will block indefinitely
    /// (see the docs on [VPromptContext]).
    pub fn yield_<'a, R: 'a>(
        &'a mut self,
        render: impl FnMut(VPromptComponentContext<'_, '_, Props, ViewData, R>) -> VNode<ViewData> + 'static
    ) -> impl Future<Output=R> + 'a {
        if let Some(deref) = self.upgrade() {
            deref.yield_(render)
        } else {
            PromptResume::pending()
        }
    }

    /// [yield_] parameterized with `R = ()`, since this is the usual case.
    pub fn yield_void<'a>(
        &'a mut self,
        render: impl FnMut(VPromptComponentContext<'_, '_, Props, ViewData, ()>) -> VNode<ViewData> + 'static
    ) -> impl Future<Output=()> + 'a {
        if let Some(deref) = self.upgrade() {
            deref.yield_void(render)
        } else {
            PromptResume::pending()
        }
    }

    fn upgrade(&mut self) -> Option<Pin<&mut PromptContextData<Props, ViewData>>> {
        if self.is_dead() {
            None
        } else if let Some(what_died) = DEAD_PROMPT_CONTEXTS.with_borrow_mut(|set|
            set.remove(&self.data_ptr())
        ) {
            assert_eq!(what_died, WhichPartOfThePromptContextDiedFirst::ContextData, "prompt context ref died twice");
            self.set_dead();
            None
        } else {
            // SAFETY: we just checked that this isn't dead, transmute seems like the only way to convert Pin ptr into Pin ref
            Some(unsafe { Pin::new_unchecked(&mut *self.0) })
        }
    }

    fn data_ptr(&self) -> *const () {
        self.0 as *const _ as *const ()
    }

    fn is_dead(&self) -> bool {
        self.0.is_null()
    }

    fn set_dead(&mut self) {
        debug_assert!(!self.is_dead());
        self.0 = null_mut();
    }
}

// region code which lets us store a weak static reference to prompt context data.
// It works because there is exactly one VPromptContext per PromptContextData,
// so when one gets dropped we insert and when the other gets dropped we remove.
// If the PromptContextData gets dropped first, we can know in the VPromptContext and "drop"
// it early: it still exists, but it knows its a dead weak reference.

thread_local! {
    static DEAD_PROMPT_CONTEXTS: RefCell<HashMap<*const (), WhichPartOfThePromptContextDiedFirst>> = RefCell::new(HashMap::new());
}

impl<Props: Any, ViewData: VViewData> Drop for VPromptContext<Props, ViewData> {
    fn drop(&mut self) {
        if !self.is_dead() {
            let data_ptr = self.data_ptr();
            DEAD_PROMPT_CONTEXTS.with_borrow_mut(|map| {
                match map.get(&data_ptr) {
                    None => {
                        map.insert(data_ptr, WhichPartOfThePromptContextDiedFirst::ContextPtrWrapper);
                    },
                    Some(WhichPartOfThePromptContextDiedFirst::ContextPtrWrapper) => {
                        panic!("VPromptContext died twice")
                    }
                    Some(WhichPartOfThePromptContextDiedFirst::ContextData) => {
                        map.remove(&data_ptr).unwrap();
                    }
                }
            });
        }
    }
}

impl<Props: Any, ViewData: VViewData> Drop for PromptContextData<Props, ViewData> {
    fn drop(&mut self) {
        let this_ptr = self as *mut _ as *const _ as *const ();
        DEAD_PROMPT_CONTEXTS.with_borrow_mut(|map| {
            match map.get(&this_ptr) {
                None => {
                    map.insert(this_ptr, WhichPartOfThePromptContextDiedFirst::ContextData);
                },
                Some(WhichPartOfThePromptContextDiedFirst::ContextData) => {
                    panic!("PromptContextData died twice")
                }
                Some(WhichPartOfThePromptContextDiedFirst::ContextPtrWrapper) => {
                    map.remove(&this_ptr).unwrap();
                }
            }
        });
    }
}
// endregion
// endregion

// region Debug boilerplate
impl<
    Props: Any,
    ViewData: VViewData,
    F: Future<Output=()>
> Debug for PromptPinned<Props, ViewData, F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VPrompt")
            .field("future_poll_fn", &"<fn(Pin<&mut F>, &mut Context<'_>) -> Poll>")
            .field("future", &"<F>")
            .field("context_data", &self.context_data)
            .finish()
    }
}

impl<Props: Any, ViewData: VViewData> Debug for PromptContextData<Props, ViewData> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PromptPinned")
            .field("current.is_some()", &self.current.is_some())
            .field("resume", &self.resume)
            .finish()
    }
}
// endregion

// region assert_is_unpin
fn assert_is_unpin<T: Unpin>(x: T) -> T {
    x
}
// endregion