//! Prompt contexts.
//! These are different than regular component [VComponentContext],
//! because regular components render instantaneously but prompt-component functions run over a lifetime.

use std::any::Any;
use std::fmt::{Debug, Formatter};
use std::future::{Future, IntoFuture, Ready, ready};
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll, RawWaker, Waker};
use crate::core::component::context::{VComponentContext1, VComponentContext2};
use crate::core::component::node::VNode;
use crate::core::view::view::VViewData;
use crate::core::misc::either_future::EitherFuture;
use crate::prompt::resume::{PromptResume, RawPromptResume};

pub struct PromptData<
    Props: Any,
    ViewData: VViewData
> {
    waker: Waker,
    current: Option<Box<dyn FnMut(VRawPromptComponentContext<'_, '_, Props, ViewData>) -> VNode<ViewData>>>,
    resume: RawPromptResume,
    phantom: PhantomData<Props>
}


pub struct VPrompt<
    Props: Any,
    ViewData: VViewData,
    F: Future<Output=()>
> {
    future: PromptFuture<F>,
    d: Pin<Box<PromptData<Props, ViewData>>>
}

pub enum PromptFuture<F: Future<Output=()>> {
    Unset,
    Pending(F),
    Ready
}

/// Context within a prompt-function. This provies [yield_], which allows you to actually render prompts.
///
/// **Note:** this type may exist longer than the actual [VPrompt] it was created from. Once the [VPrompt] is destroyed,
/// the context will simply block forever the next time you try to yield anything.
pub struct VPromptContext<Props: Any, ViewData: VViewData>(*mut dyn _VPromptContext<Props, ViewData>);

impl<Props: Any, ViewData: VViewData, F: Future<Output=()>> !Sync for VPrompt<Props, ViewData, F> {}
impl<Props: Any, ViewData: VViewData, F: Future<Output=()>> !Send for VPrompt<Props, ViewData, F> {}

type VRawPromptComponentContext<'a, 'a0, Props, ViewData> = (VComponentContext1<'a, 'a0, Props, ViewData>, &'a mut RawPromptResume, &'a Props);
pub type VPromptComponentContext<'a, 'a0, Props, ViewData, R> = (VComponentContext1<'a, 'a0, Props, ViewData>, PromptResume<'a, R>, &'a Props);
pub type VPromptContext2<Props, ViewData, PromptProps> = (VPromptContext<Props, ViewData>, PromptProps);

trait _VPromptContext<Props: Any, ViewData: VViewData> {
    unsafe fn yield_raw<'a>(
        &'a mut self,
        render: Box<dyn FnMut(VRawPromptComponentContext<'_, '_, Props, ViewData>) -> VNode<ViewData>>
    ) -> &'a mut RawPromptResume;
}

impl<Props: Any, ViewData: VViewData> dyn _VPromptContext<Props, ViewData> + '_ {
    pub fn yield_<'a, R>(
        &'a mut self,
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
        &'a mut self,
        render: impl FnMut(VPromptComponentContext<'_, '_, Props, ViewData, ()>) -> VNode<ViewData> + 'static
    ) -> PromptResume<'a, ()> {
        self.yield_(render)
    }
}

impl<Props: Any, ViewData: VViewData> VPromptContext<Props, ViewData> {
    fn new(inner: &mut (dyn _VPromptContext<Props, ViewData> + 'static)) -> Self {
        VPromptContext(inner as *mut _)
    }

    fn upgrade(&mut self) -> Option<&mut dyn _VPromptContext<Props, ViewData>> {
        if self.0.is_null() {
            None
        } else {
            Some(unsafe { &mut *self.0 })
        }
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
}

impl<
    Props: Any,
    ViewData: VViewData + 'static,
    F: Future<Output=()> + 'static
> VPrompt<Props, ViewData, F> {
    pub fn new<PromptProps>(prompt_fn: impl FnOnce(VPromptContext2<Props, ViewData, PromptProps>) -> F, prompt_props: PromptProps) -> Self {
        let mut this = Self {
            waker: Self::prompt_waker(),
            future: PromptFuture::Unset,
            current: None,
            resume: RawPromptResume::new(),
            phantom: PhantomData
        };
        let future = prompt_fn((VPromptContext::new(&mut this), prompt_props));
        this.future = PromptFuture::Pending(future);
        this.poll_future();
        assert!(this.current.is_some(), "prompt functions must yield something before awaiting. Yield a \"loading\" or empty component if you're not ready");
        this
    }
}

impl<
    Props: Any,
    ViewData: VViewData,
    F: Future<Output=()>
> VPrompt<Props, ViewData, F> {
    fn prompt_waker() -> Waker {
        let raw = RawWaker::new();
        unsafe { Waker::from_raw(raw) }
    }

    fn poll_future(&mut self) {
        if let PromptFuture::Pending(future) = &mut self.future {
            if let Poll::Ready(()) = future.poll(Context::from_waker(self.waker)) {
                self.future = PromptFuture::Ready;
            }
        }
    }

    pub fn current(&mut self, (c, props): VComponentContext2<'_, '_, Props, ViewData>) -> VNode<ViewData> {
        self.poll_future();
        let current = self.current.as_mut().expect("prompt is still being created, you can't get current component yet");
        current((c, &mut self.resume, props))
    }
}

impl<
    Props: Any,
    ViewData: VViewData,
    F: Future<Output=()>
> _VPromptContext<Props, ViewData> for VPrompt<Props, ViewData, F> {
    unsafe fn yield_raw<'a>(&'a mut self, render: Box<dyn FnMut(VRawPromptComponentContext<'_, '_, Props, ViewData>) -> VNode<ViewData>>) -> &'a mut RawPromptResume {
        self.current = Some(render);
        &mut self.resume
    }
}

impl<
    Props: Any,
    ViewData: VViewData,
    F: Future<Output=()>
> IntoFuture for VPrompt<Props, ViewData, F> {
    type Output = ();
    type IntoFuture = EitherFuture<F, Ready<()>, ()>;

    /// Returns a future which will complete when the wrapped prompt function does.
    fn into_future(self) -> Self::IntoFuture {
        match self.future {
            PromptFuture::Unset => panic!("prompt is still being created, you can't await it yet"),
            PromptFuture::Pending(f) => EitherFuture::Left(f),
            PromptFuture::Ready => EitherFuture::Right(ready(()))
        }
    }
}

impl<
    Props: Any,
    ViewData: VViewData,
    F: Future<Output=()>
> Debug for VPrompt<Props, ViewData, F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VPrompt")
            .field("future", &self.future)
            .field("current.is_some()", &self.current.is_some())
            .field("resume", &self.resume)
            .finish()
    }
}

impl<F: Future<Output=()>> Debug for PromptFuture<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PromptFuture::Unset => write!(f, "PromptFuture::Unset"),
            PromptFuture::Pending(_future) => write!(f, "PromptFuture::Pending(...)"),
            PromptFuture::Ready => write!(f, "PromptFuture::Ready")
        }
    }
}