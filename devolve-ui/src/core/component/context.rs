use std::any::Any;
use std::cell::{RefCell, Ref, RefMut};
use std::ops::Deref;
use std::rc::{Rc, Weak};
use crate::core::component::component::VComponent;
use crate::core::renderer::engine::RenderEngine;
use crate::core::renderer::renderer::{Renderer, VRender};
use crate::core::view::layout::geom::{BoundingBox, Rectangle, Size};
use crate::core::view::layout::parent_bounds::ParentBounds;
use crate::core::view::view::{VView, VViewData, VViewType};

pub struct VContext {
    renderer: Weak<dyn Any>,
    components: Vec<RefCell<Box<dyn Any>>>,
}

impl VContext {
    fn with_global<R>(f: impl FnOnce(Ref<VContext>) -> R) -> R {
        CONTEXT.with(|context: &RefCell<VContext>| f(context.borrow()))
    }

    fn with_global_mut<R>(f: impl FnOnce(RefMut<Engine>) -> R) -> R {
        CONTEXT.with(|context: &RefCell<Engine>| f(context.borrow_mut()))
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

    pub fn get_renderer<Engine : RenderEngine>() -> Rc<Renderer<Engine>> {
        Self::with_global(|this| this
            .renderers
            .last()
            .expect("no renderers in context")
            .upgrade()
            .expect("renderer in context was freed")
            .downcast::<Renderer<Engine>>()
            .expect("renderer in context was not of expected parameterized type"))
    }

    pub fn has_component() -> bool {
        Self::with_global(|this| !this.components.is_empty())
    }

    pub fn with_top_component<R, ViewData: VViewData>(f: impl FnOnce(RefMut<Box<VComponent<ViewData>>>) -> R) -> R {
        Self::with_global_subref_mut(|this| this
            .components
            .last()
            .expect("no components in context")
            .downcast::<VComponent<ViewData>>()
            .expect("component in context was not of expected parameterized type"), f)
    }

    pub unsafe fn with_top_component_unsafe<R, ViewData: VViewData>(f: impl FnOnce(&mut Box<VComponent<ViewData>>) -> R) -> R {
        Self::with_global_subref_unsafe(|this| this
            .components
            .last()
            .expect("no components in context")
            .downcast::<VComponent<ViewData>>()
            .expect("component in context was not of expected parameterized type"), f)
    }

    pub fn with_try_top_component<R, ViewData: VViewData>(f: impl FnOnce(Option<RefMut<Box<VComponent<ViewData>>>>) -> R) -> R {
        Self::with_global_opt_subref_mut(|this| this
            .components
            .last()
            .map(|component| component
                .downcast::<VComponent<ViewData>>()
                .expect("component in context was not of expected parameterized type")), f)
    }

    /* pub fn with_iter_components_top_down<R>(f: impl FnOnce(RefMut<impl Iterator<Item=&mut Box<VComponent<Engine::ViewData>>>>>) -> R) -> R {
        Self::with_global(|this| this
            .components
            .iter_mut()
            .rev(), f)
    } */

    pub fn with_push_renderer<R, Engine: RenderEngine>(renderer: Weak<Renderer<Engine>>, f: impl FnOnce() -> R) -> R {
        // We need to not borrow during f or we'll get a RefCell runtime error
        Self::with_global_mut(|mut this| this.renderers.push(renderer));
        let result = f();
        Self::with_global_mut(|mut this| this.renderers.pop().expect("renderer stack misaligned: empty context when trying to pop renderer"));
        result
    }

    pub fn with_push_component<R, ViewData: VViewData>(component: Box<VComponent<ViewData>>, f: impl FnOnce() -> R) -> (R, Box<VComponent<Engine::ViewData>>) {
        // We need to not borrow during f or we'll get a RefCell runtime error
        Self::with_global_mut(|mut this| this.components.push(RefCell::new(component)));
        let result = f();
        let component = Self::with_global_mut(|mut this| this.components.pop().expect("component stack misaligned: empty context when trying to pop component")).into_inner();
        (result, component)
    }

    pub fn with_local_context<R>(f: impl FnOnce() -> R) -> R {
        // We need to not borrow during f or we'll get a RefCell runtime error
        let num_old_components = Self::with_global(|this| this.components.len());
        let result = f();
        assert!(Self::with_global(|this| this.components.len() == num_old_components), "component stack misaligned after local context");
        Self::with_global_mut(|mut this| this.components.append(&mut old_components));
        result
    }
}

thread_local! {
    static CONTEXT: RefCell<VContext> = RefCell::new();
}