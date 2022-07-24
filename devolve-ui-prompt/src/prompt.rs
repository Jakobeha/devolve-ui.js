use std::any::Any;
use std::future::{Future, IntoFuture, Ready, ready};
use std::marker::PhantomData;
use std::cell::RefCell;
use std::pin::Pin;
use std::fmt::{Debug, Formatter};
use std::ptr::addr_of_mut;
use crate::component::context::{VComponentContext2, VContext};
use crate::component::node::VNode;
use crate::component::path::VComponentRef;
use crate::hooks::state_internal::NonUpdatingStateHook;
use crate::misc::either_future::EitherFuture;
use crate::view::view::VViewData;
use crate::prompt::waker::PromptWaker;
use crate::prompt::context::{PromptContextData, VPromptContext, VPromptContext2};
use crate::prompt::misc::assert_is_unpin;
use crate::prompt::resume::RawPromptResume;

pub fn prompt_fn_into_component_fn<PromptProps, Props: Any, ViewData: VViewData + 'static, F: Future<Output=()> + 'static>(
    prompt_fn: impl Fn(VPromptContext2<Props, ViewData, PromptProps>) -> F + 'static,
    get_prompt_props: impl Fn() -> PromptProps + 'static
) -> impl Fn(VComponentContext2<Props, ViewData>) -> VNode<ViewData> + 'static {
    move |(mut c, props)| {
        let c2_idx = c.use_non_updating_state(|_c| VPrompt::new(&prompt_fn, get_prompt_props()));
        // Idk if having a mutable borrow on c2 and on c is sound according to Rust's rules,
        // but c passed to c2.current is guaranteed not to access c2 again (c2_idx is local and you can't soundly re-create it),
        // so it should be fine in practice.
        //
        // Ideally there would be a workaround but I don't think this is trivial.
        // We can't just use interior mutability because we need a mutable borrow of c anyways.
        // We could store c2 in a separate structure like a thread_local vector or hashmap
        // and non_updating_state would store the key, but I don't think it's worth it.
        let c2 = unsafe { &mut *(&mut c[c2_idx] as *mut VPrompt<Props, ViewData, F>) };
        c2.set_component_ref(c.component().vref());
        c2.current((c, props))
    }
}

pub struct VPrompt<
    Props: Any,
    ViewData: VViewData,
    F: Future<Output=()>
>(Pin<Box<PromptPinned<Props, ViewData, F>>>);

pub struct PromptPinned<
    Props: Any,
    ViewData: VViewData,
    F: Future<Output=()>
> {
    pub(super) poll_future: Box<dyn Fn()>,
    future: RefCell<Option<F>>,
    context_data: PromptContextData<Props, ViewData>
}

impl<
    Props: Any,
    ViewData: VViewData + 'static,
    F: Future<Output=()> + 'static
> VPrompt<Props, ViewData, F> {
    pub fn new<PromptProps>(prompt_fn: impl FnOnce(VPromptContext2<Props, ViewData, PromptProps>) -> F, prompt_props: PromptProps) -> Self {
        // Setup uninit addresses
        let mut pinned = Box::<PromptPinned<Props, ViewData, F>>::new_uninit();
        let poll_future = unsafe { addr_of_mut!((*pinned.as_mut_ptr()).poll_future) };
        let future = unsafe { addr_of_mut!((*pinned.as_mut_ptr()).future) };
        let context_data = unsafe { addr_of_mut!((*pinned.as_mut_ptr()).context_data) };

        // poll_future is statically known
        unsafe {
            poll_future.write(Box::new(move || PromptWaker::poll(poll_future, future)));
        };

        // Setup context data
        let the_context_data = PromptContextData {
            current: None,
            current_ref: None,
            resume: RawPromptResume::new(),
            phantom: PhantomData
        };
        unsafe { context_data.write(the_context_data); }
        let context = VPromptContext::new(context_data);

        // Get future with pinned setup context data data
        let the_future = prompt_fn((context, prompt_props));
        unsafe { future.write(RefCell::new(Some(the_future))) };

        // Poll the future once, PromptWaker will take care of future polling
        unsafe { PromptWaker::wake(poll_future); }

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
    fn set_component_ref(&mut self, component_ref: VComponentRef<ViewData>) {
        // SAFETY: we aren't moving any of the data in this
        let pinned = unsafe { self.0.as_mut().get_unchecked_mut() };
        pinned.context_data.current_ref = Some(component_ref);
    }

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
