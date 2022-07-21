use std::any::Any;
use std::future::Future;
use crate::core::view::view::VViewData;
use crate::core::component::context::{VComponentContext2, VComponentContext};
use crate::core::component::node::VNode;
use crate::prompt::context::{VPrompt, VPromptContext2};

pub mod context;
pub mod resume;

pub fn prompt_fn_into_component_fn<PromptProps, Props: Any, ViewData: VViewData, F: Future<Output=()>>(
    prompt_fn: impl FnOnce(VPromptContext2<'_, PromptProps, Props, ViewData>) -> F + 'static,
    prompt_props: PromptProps,
) -> impl Fn(VComponentContext2<Props, ViewData>) -> VNode<ViewData> + 'static {
    let mut c2 = VPrompt::new(prompt_fn, prompt_props);
    RefFn::new(move |c| {
        c2.current(c)
    })
}