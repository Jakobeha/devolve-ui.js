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
    get_prompt_props: impl Fn() -> PromptProps,
) -> impl Fn(VComponentContext2<Props, ViewData>) -> VNode<ViewData> + 'static {
    move |(mut c, props)| {
        let c2_idx = c.use_non_updating_state(|_c| VPrompt::new(prompt_fn, get_prompt_props()));
        // Idk if having a mutable borrow on c2 and on c is sound according to Rust's rules,
        // but c passed to c2.current is guaranteed not to access c2 again (c2_idx is local and you can't soundly re-create it),
        // so it should be fine in practice.
        //
        // Ideally there would be a workaround but I don't think this is trivial.
        // We can't just use interior mutability because we need a mutable borrow of c anyways.
        // We could store c2 in a separate structure like a thread_local vector or hashmap
        // and non_updating_state would store the key, but I don't think it's worth it.
        let mut c2 = unsafe { &mut *(&mut c[c2_idx] as *mut VPrompt<Props, ViewData, _>) };
        c2.current((c, props))
    }
}