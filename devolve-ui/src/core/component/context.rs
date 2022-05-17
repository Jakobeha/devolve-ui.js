use std::cell::{RefCell, RefMut};
use std::rc::{Rc, Weak};
use crate::core::component::component::VComponent;
use crate::renderer::Renderer;

pub struct VContext {
    renderers: Vec<Weak<Renderer>>,
    components: Vec<Box<VComponent>>,
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
            .expect("renderer in context was freed"))
    }

    pub fn has_component() -> bool {
        Self::borrow_global(|this| !this.components.is_empty())
    }

    pub fn get_component() -> &'static mut Box<VComponent> {
        Self::borrow_global(|this| this
            .components
            .last_mut()
            .expect("no components in context"))
    }

    pub fn try_get_component() -> Option<&'static mut Box<VComponent>> {
        Self::borrow_global(|this| this
            .components
            .last_mut())
    }

    pub fn iter_components_top_down() -> impl Iterator<Item=&'static mut Box<VComponent>> {
        Self::borrow_global(|this| this
            .components
            .iter_mut()
            .rev())
    }

    pub fn with_renderer<R>(renderer: Weak<Renderer>, f: impl FnOnce() -> R) -> R {
        // We need to not borrow during f or we'll get a RefCell runtime error
        Self::borrow_global(|this| this.renderers.push(renderer));
        let result = f();
        Self::borrow_global(|this| this.renderers.pop());
        result
    }

    pub fn with_component<R>(component: Box<VComponent>, f: impl FnOnce() -> R) -> (R, Box<VComponent>) {
        // We need to not borrow during f or we'll get a RefCell runtime error
        Self::borrow_global(|this| this.components.push(component));
        let result = f();
        let component = Self::borrow_global(|this| this.components.pop().expect("component stack misaligned"));
        (result, component)
    }

    pub fn with_empty_component_stack<R>(f: impl FnOnce() -> R) -> R {
        // We need to not borrow during f or we'll get a RefCell runtime error
        let mut old_components = Self::borrow_global(|this| {
            let mut old_components = Vec::new();
            old_components.append(&mut this.components);
            old_components
        });
        let result = f();
        assert!(Self::borrow_global(|this| this.components.is_empty()), "component stack misaligned");
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