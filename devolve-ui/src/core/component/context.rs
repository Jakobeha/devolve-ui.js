use std::borrow::BorrowMut;
use std::cell::{RefCell, RefMut};
use std::rc::Weak;
use std::sync::Mutex;
use crate::core::component::component::VComponent;
use crate::renderer::Renderer;

pub struct VContext {
    renderers: Vec<Weak<Renderer>>,
    components: Vec<Weak<VComponent>>,
}

impl VContext {
    fn borrow_global<R>(f: impl FnOnce(RefMut<VContext>) -> R) -> R {
        CONTEXT.with(|mut context: RefCell<VContext>| f(context.borrow_mut()))
    }

    pub fn get_renderer() -> Weak<Renderer> {
        Self::borrow_global(|this| this.renderers.last_mut().expect("no renderers in context").clone())
    }

    pub fn get_component() -> Weak<VComponent> {
        Self::borrow_global(|this| this.components.last_mut().expect("no components in context").clone())
    }

    pub fn iter_components_top_down() -> impl Iterator<Item=Weak<VComponent>> {
        Self::borrow_global(|this| this.components.clone().into_iter().rev())
    }

    pub fn with_renderer(renderer: Weak<Renderer>, f: impl FnOnce() -> R) -> R {
        // We need to not borrow during f or we'll get a RefCell runtime error
        Self::borrow_global(|this| this.renderers.push(renderer));
        let result = f();
        Self::borrow_global(|this| this.renderers.pop());
        result
    }

    pub fn with_component(component: Weak<VComponent>, f: impl FnOnce() -> R) -> R {
        // We need to not borrow during f or we'll get a RefCell runtime error
        Self::borrow_global(|this| this.components.push(component));
        let result = f();
        Self::borrow_global(|this| this.components.pop());
        result
    }
}

thread_local! {
    static CONTEXT: RefCell<VContext> = RefCell::new(VContext {
        renderers: Vec::new(),
        components: Vec::new(),
    });
}