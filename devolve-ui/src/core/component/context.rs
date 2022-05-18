use std::cell::{RefCell, Ref, RefMut};
use std::ops::Deref;
use std::rc::{Rc, Weak};
use crate::core::component::component::VComponent;
use crate::renderer::Renderer;

pub struct VContext {
    renderers: Vec<Weak<Renderer>>,
    components: Vec<RefCell<Box<VComponent>>>,
}

impl VContext {
    fn with_global<R>(f: impl FnOnce(Ref<VContext>) -> R) -> R {
        CONTEXT.with(|context: &RefCell<VContext>| f(context.borrow()))
    }

    fn with_global_mut<R>(f: impl FnOnce(RefMut<VContext>) -> R) -> R {
        CONTEXT.with(|context: &RefCell<VContext>| f(context.borrow_mut()))
    }

    fn with_global_subref_mut<R, UserR>(f: impl FnOnce(&VContext) -> &RefCell<R>, user_f: impl FnOnce(RefMut<R>) -> UserR) -> UserR {
        Self::with_global(|context| user_f(f(Ref::deref(&context)).borrow_mut()))
    }

    fn with_global_opt_subref_mut<R, UserR>(f: impl FnOnce(&VContext) -> Option<&RefCell<R>>, user_f: impl FnOnce(Option<RefMut<R>>) -> UserR) -> UserR {
        Self::with_global(|context| user_f(f(Ref::deref(&context)).map(|subref| subref.borrow_mut())))
    }

    unsafe fn with_global_subref_unsafe<R, UserR>(f: impl FnOnce(&VContext) -> &RefCell<R>, user_f: impl FnOnce(&mut R) -> UserR) -> UserR {
        Self::with_global(|context| user_f(f(Ref::deref(&context)).as_ptr().as_mut().unwrap()))
    }

    pub fn get_renderer() -> Rc<Renderer> {
        Self::with_global(|this| this
            .renderers
            .last()
            .expect("no renderers in context")
            .upgrade()
            .expect("renderer in context was freed"))
    }

    pub fn has_component() -> bool {
        Self::with_global(|this| !this.components.is_empty())
    }

    pub fn with_top_component<R>(f: impl FnOnce(RefMut<Box<VComponent>>) -> R) -> R {
        Self::with_global_subref_mut(|this| this
            .components
            .last()
            .expect("no components in context"), f)
    }

    pub unsafe fn with_top_component_unsafe<R>(f: impl FnOnce(&mut Box<VComponent>) -> R) -> R {
        Self::with_global_subref_unsafe(|this| this
            .components
            .last()
            .expect("no components in context"), f)
    }

    pub fn with_try_top_component<R>(f: impl FnOnce(Option<RefMut<Box<VComponent>>>) -> R) -> R {
        Self::with_global_opt_subref_mut(|this| this
            .components
            .last(), f)
    }

    /* pub fn with_iter_components_top_down<R>(f: impl FnOnce(RefMut<impl Iterator<Item=&mut Box<VComponent>>>) -> R) -> R {
        Self::with_global(|this| this
            .components
            .iter_mut()
            .rev(), f)
    } */

    pub fn with_push_renderer<R>(renderer: Weak<Renderer>, f: impl FnOnce() -> R) -> R {
        // We need to not borrow during f or we'll get a RefCell runtime error
        Self::with_global_mut(|mut this| this.renderers.push(renderer));
        let result = f();
        Self::with_global_mut(|mut this| this.renderers.pop());
        result
    }

    pub fn with_push_component<R>(component: Box<VComponent>, f: impl FnOnce() -> R) -> (R, Box<VComponent>) {
        // We need to not borrow during f or we'll get a RefCell runtime error
        Self::with_global_mut(|mut this| this.components.push(RefCell::new(component)));
        let result = f();
        let component = Self::with_global_mut(|mut this| this.components.pop().expect("component stack misaligned")).into_inner();
        (result, component)
    }

    pub fn with_clear_component_stack<R>(f: impl FnOnce() -> R) -> R {
        // We need to not borrow during f or we'll get a RefCell runtime error
        let mut old_components = Self::with_global_mut(|mut this| {
            let mut old_components = Vec::new();
            old_components.append(&mut this.components);
            old_components
        });
        let result = f();
        assert!(Self::with_global(|this| this.components.is_empty()), "component stack misaligned");
        Self::with_global_mut(|mut this| this.components.append(&mut old_components));
        result
    }
}

thread_local! {
    static CONTEXT: RefCell<VContext> = RefCell::new(VContext {
        renderers: Vec::new(),
        components: Vec::new(),
    });
}