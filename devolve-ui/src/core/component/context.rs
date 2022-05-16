use std::borrow::BorrowMut;
use std::cell::{RefCell, RefMut};
use std::rc::{Rc, Weak};
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

    pub fn get_renderer() -> Rc<Renderer> {
        Self::borrow_global(|this| this
            .renderers
            .last_mut()
            .expect("no renderers in context")
            .upgrade()
            .expect("renderer shouldn't have been freed"))
    }

    pub fn has_component() -> bool {
        Self::borrow_global(|this| !this.components.is_empty())
    }

    pub fn get_component() -> Rc<VComponent> {
        Self::borrow_global(|this| this
            .components
            .last_mut()
            .expect("no components in context")
            .upgrade()
            .expect("component shouldn't have been freed"))
    }

    pub fn try_get_component() -> Option<Rc<VComponent>> {
        Self::borrow_global(|this| this
            .components
            .last_mut()
            .map(|weak| weak.upgrade().expect("component shouldn't have been freed")))
    }

    pub fn iter_components_top_down() -> impl Iterator<Item=Rc<VComponent>> {
        Self::borrow_global(|this| this
            .components
            .clone()
            .into_iter()
            .rev()
            .map(|weak| weak.upgrade().expect("component shouldn't have been freed")))
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

    pub fn with_empty_component_stack(f: impl FnOnce() -> R) -> R {
        // We need to not borrow during f or we'll get a RefCell runtime error
        let mut old_components = Self::borrow_global(|this| this.components.clone());
        Self::borrow_global(|this| this.components.clear());
        let result = f();
        assert!(Self::borrow_global(|this| this.components.is_empty()), "component stack mismatch");
        Self::borrow_global(|this| this.components.append(&mut old_components));
        result
    }
}

thread_local! {
    static CONTEXT: RefCell<VContext> = RefCell::new(VContext {
        renderers: Vec::new(),
        components: Vec::new(),
    });
}