//! Prompt contexts.
//! These are different than regular component [VComponentContext],
//! because regular components render instantaneously but prompt-component functions run over a lifetime.

use std::any::Any;
use std::cell::Cell;
use std::future::{Future, IntoFuture};
use std::marker::PhantomData;
use crate::core::component::component::{VComponentContexts, VComponentHead};
use crate::core::component::context::{VComponentContext1, VComponentContext2};
use crate::core::component::node::VNode;
use crate::core::view::view::{VView, VViewData};
use crate::prompt::resume::{PromptResume, RawPromptResume};

#[derive(Debug)]
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
pub type VPromptContext2<'a, PromptProps, Props, ViewData> = (&'a mut dyn VPromptContext<Props, ViewData>, PromptProps);

pub trait VPromptContext<Props: Any, ViewData: VViewData> {
    fn yield_<'a, R>(
        &'a mut self,
        render: impl FnMut(VPromptComponentContext<'_, '_, Props, ViewData, R>) -> VNode<ViewData>
    ) -> &'a mut PromptResume<'a, R>;
}

impl<
    Props: Any,
    ViewData: VViewData,
    F: Future<Output=()>
> VPrompt<Props, ViewData, F> {
    pub fn new<PromptProps>(prompt_fn: impl FnOnce(VPromptContext2<'_, PromptProps, Props, ViewData>) -> F + 'static, prompt_props: PromptProps) -> Self {
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
        let current = self.current.expect("prompt is still being created, you can't get current component yet");
        current((c, &mut self.resume, props))
    }
}

impl<
    Props: Any,
    ViewData: VViewData,
    F: Future<Output=()>
> VPromptContext<Props, ViewData> for VPrompt<Props, ViewData, F> {
    fn yield_<'a, R>(&'a mut self, render: impl FnMut(VPromptComponentContext<'_, '_, Props, ViewData, R>) -> VNode<ViewData>) -> &'a mut PromptResume<'a, R> {
        let mut resume = PromptResume::new(&mut self.resume_shared);
        self.current = Some(Box::new(|(c, resume, props)| {
            let resume = PromptResume::new(resume);
            render((c, resume, props))
        }));
        &mut resume
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