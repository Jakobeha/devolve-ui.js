//! Prompt contexts.
//! These are different than regular component [VComponentContext],
//! because regular components render instantaneously but prompt-component functions run over a lifetime.

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::ptr::null_mut;
use crate::core::component::context::VComponentContext1;
use crate::core::component::node::VNode;
use crate::core::component::path::VComponentRef;
use crate::core::view::view::VViewData;
use crate::prompt::misc;
use crate::prompt::resume::{PromptResume, RawPromptResume};

/// Context within a prompt-function. This provies [yield_], which allows you to actually render prompts.
///
/// **Note:** this type may exist longer than the actual [VPrompt] it was created from. Once the [VPrompt] is destroyed,
/// the context will simply block forever the next time you try to yield anything.
pub struct VPromptContext<Props: Any, ViewData: VViewData>(*mut PromptContextData<Props, ViewData>);

pub type VPromptComponentContext<'a, 'a0, Props, ViewData, R> = (VComponentContext1<'a, 'a0, Props, ViewData>, PromptResume<'a, R>, &'a Props);
pub type VPromptContext2<Props, ViewData, PromptProps> = (VPromptContext<Props, ViewData>, PromptProps);
type VRawPromptComponentContext<'a, 'a0, Props, ViewData> = (VComponentContext1<'a, 'a0, Props, ViewData>, &'a mut RawPromptResume, &'a Props);

pub(super) struct PromptContextData<
    Props: Any,
    ViewData: VViewData
> {
    pub(super) current: Option<Box<dyn FnMut(VRawPromptComponentContext<'_, '_, Props, ViewData>) -> VNode<ViewData>>>,
    pub(super) current_ref: Option<VComponentRef<ViewData>>,
    pub(super) resume: RawPromptResume,
    pub(super) phantom: PhantomData<Props>
}

#[derive(Debug, PartialEq, Eq)]
enum WhichPartOfThePromptContextDiedFirst {
    ContextData,
    ContextPtrWrapper
}

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
        if let Some(current_ref) = this.current_ref.take() {
            current_ref.pending_update("prompt-yield")
        }
        misc::assert_is_unpin(&mut this.resume)
    }
}

// Absolutely nothing here is thread-safe
impl<Props: Any, ViewData: VViewData> !Sync for PromptContextData<Props, ViewData> {}
impl<Props: Any, ViewData: VViewData> !Send for PromptContextData<Props, ViewData> {}

impl<Props: Any, ViewData: VViewData> VPromptContext<Props, ViewData> {
    pub(super) fn new(context_data: *mut PromptContextData<Props, ViewData>) -> Self {
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

impl<Props: Any, ViewData: VViewData> Debug for PromptContextData<Props, ViewData> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PromptPinned")
            .field("current.is_some()", &self.current.is_some())
            .field("resume", &self.resume)
            .finish()
    }
}