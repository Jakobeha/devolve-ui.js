//! Prompt contexts.
//! These are different than regular component [VComponentContext],
//! because regular components render instantaneously but prompt-component functions run over a lifetime.

use std::any::Any;
use std::fmt::{Debug, Formatter};
use std::future::{Future, IntoFuture};
use std::marker::PhantomData;
use crate::core::component::context::{VComponentContext1, VComponentContext2};
use crate::core::component::node::VNode;
use crate::core::view::view::VViewData;
use crate::prompt::resume::{PromptResume, RawPromptResume};

pub struct VPrompt<
    Props: Any,
    ViewData: VViewData,
    F: Future<Output=()>
> {
    future: Option<F>,
    current: Option<Box<dyn FnMut(VRawPromptComponentContext<'_, '_, Props, ViewData>) -> VNode<ViewData>>>,
    resume: RawPromptResume,
    phantom: PhantomData<Props>
}

type VRawPromptComponentContext<'a, 'a0, Props, ViewData> = (VComponentContext1<'a, 'a0, Props, ViewData>, &'a mut RawPromptResume, &'a Props);
pub type VPromptComponentContext<'a, 'a0, Props, ViewData, R> = (VComponentContext1<'a, 'a0, Props, ViewData>, PromptResume<'a, R>, &'a Props);
pub type VPromptContext2<'a, Props, ViewData, PromptProps> = (&'a mut (dyn VPromptContext<Props, ViewData> + 'a), PromptProps);

pub trait VPromptContext<Props: Any, ViewData: VViewData> {
    unsafe fn yield_raw<'a>(
        &'a mut self,
        render: Box<dyn FnMut(VRawPromptComponentContext<'_, '_, Props, ViewData>) -> VNode<ViewData>>
    ) -> &'a mut RawPromptResume;
}

impl<Props: Any, ViewData: VViewData> dyn VPromptContext<Props, ViewData> + '_ {
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

impl<
    Props: Any,
    ViewData: VViewData,
    F: Future<Output=()>
> VPrompt<Props, ViewData, F> {
    pub fn new<PromptProps>(prompt_fn: impl FnOnce(VPromptContext2<'_, Props, ViewData, PromptProps>) -> F, prompt_props: PromptProps) -> Self {
        let mut this = Self {
            future: None,
            current: None,
            resume: RawPromptResume::new(),
            phantom: PhantomData
        };
        let future = prompt_fn((&mut this, prompt_props));
        assert!(this.current.is_some(), "prompt functions must yield something before awaiting. Yield a \"loading\" or empty component if you're not ready");
        this.future = Some(future);
        this
    }

    pub fn current(&mut self, (c, props): VComponentContext2<'_, '_, Props, ViewData>) -> VNode<ViewData> {
        let current = self.current.as_mut().expect("prompt is still being created, you can't get current component yet");
        current((c, &mut self.resume, props))
    }
}

impl<
    Props: Any,
    ViewData: VViewData,
    F: Future<Output=()>
> VPromptContext<Props, ViewData> for VPrompt<Props, ViewData, F> {
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
    type IntoFuture = F;

    /// Returns a future which will complete when the wrapped prompt function does.
    fn into_future(self) -> Self::IntoFuture {
        self.future.expect("prompt is still being created, you can't await it yet")
    }
}

impl<
    Props: Any,
    ViewData: VViewData,
    F: Future<Output=()>
> Debug for VPrompt<Props, ViewData, F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VPrompt")
            .field("future.is_some()", &self.future.is_some())
            .field("current.is_some()", &self.current.is_some())
            .field("resume", &self.resume)
            .finish()
    }
}