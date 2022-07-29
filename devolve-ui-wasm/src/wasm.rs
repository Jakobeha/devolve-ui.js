use js_sys;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use devolve_ui::component::constr::make_component;
use devolve_ui::component::context::{VComponentContext1, VComponentContextUnsafe};
use devolve_ui::component::node::{NodeId, VNode};
use devolve_ui::view::layout::parent_bounds::SubLayout;
use devolve_ui::view::view::{VViewData, VViewType};

#[wasm_bindgen(typescript_custom_section)]
const NODE_TYPE: &'static str = r"
type Node = any;
type Component<OptionalProps extends Object, RequiredProps extends object> = {
    _internal: (key: string, props: Partial<OptionalProps> & RequiredProps) => Node
};
";

#[wasm_bindgen(typescript_custom_section)]
const DEFINE_COMPONENT: &'static str = r"
export function define_component<OptionalProps extends object, RequiredProps extends object>(
    fun: (props: OptionalProps & RequiredProps) => Node,
    optional_prop_defaults: OptionalProps
): Component<OptionalProps, RequiredProps>;
";

struct JsViewData {

}

impl VViewData for JsViewData {
    type Children<'a> where Self: 'a = ();
    type ChildrenMut<'a> where Self: 'a = ();

    fn typ(&self) -> VViewType {
        todo!()
    }

    fn children(&self) -> Option<(Self::Children<'_>, SubLayout)> {
        todo!()
    }

    fn children_mut(&mut self) -> Option<(Self::ChildrenMut<'_>, SubLayout)> {
        todo!()
    }
}

thread_local! {
    static CONTEXTS: RefCell<Vec<VComponentContextUnsafe>> = RefCell::new(Vec::new());
    static NODES: RefCell<HashMap<NodeId, VNode<JsViewData>>> = RefCell::new(HashMap::new());
}

struct NoTopContextError;
struct MissingNodeError;
enum JsToVNodeError {
    InvalidArgument,
    Occupied
}

fn with_top_context<R, F: FnOnce(VComponentContext1<'_, '_, js_sys::Object, JsViewData>) -> R>(fun: F) -> Result<R, NoTopContextError> {
    CONTEXTS.with_borrow(|contexts| {
        let context = *contexts.last().ok_or(NoTopContextError)?;
        let context = unsafe { VComponentContext1::from_unsafe(context) };
        Ok(fun(context))
    })
}

fn with_push_context<R, F: FnOnce() -> R>(context: &mut VComponentContext1<'_, '_, js_sys::Object, JsViewData>, fun: F) -> R {
    CONTEXTS.with_borrow_mut(|contexts| contexts.push(context.as_unsafe()));
    let result = fun();
    CONTEXTS.with_borrow_mut(|contexts| contexts.pop().unwrap());
    result
}

fn vnode_to_js(node: VNode<JsViewData>) -> Result<JsValue, OccupiedNodeError> {
    NODES.with_borrow_mut(|nodes| nodes.try_insert(node.id(), node).map_err(|_err| OccupiedNodeError))?;
    Ok(JsValue::from(node.id()))
}

fn js_to_vnode(js: JsValue) -> Result<VNode<JsViewData>, js_sys::Error> {
    let node_id: f64 = js.as_f64().map_err(|_err| JsToVNodeError::InvalidArgument)?;
    if node_id.floor() != node_id {
        return Err(JsToVNodeError::InvalidArgument);
    }
    let node_id = NodeId(node_id as usize);

    NODES.with_borrow_mut(|nodes| nodes.remove(&node_id).ok_or(JsToVNodeError::Occupied))
}

fn collapse_vnode(vnode: Result<VNode<JsViewData>, js_sys::Error>) -> VNode<JsViewData> {
    match vnode {
        Err(err) => todo!(err),
        Ok(node) => node
    }
}

#[wasm_bindgen(skip_typescript)]
pub fn define_component(fun: js_sys::Function, optional_prop_defaults: js_sys::Object) -> JsValue {
    Closure::<dyn Fn(js_sys::JsString, js_sys::Object) -> Result<JsValue, JsValue>>::new(move |key: js_sys::JsString, props: js_sys::Object| {
        let filled_props = js_sys::Object::new();
        js_sys::Object::assign2(&filled_props, &optional_prop_defaults, &props);
        with_top_context(move |mut c| {
            vnode_to_js(make_component(&mut c, &key, filled_props, move |(mut c, filled_props)| {
                with_push_context(&mut c, move || {
                    collapse_vnode(js_to_vnode(js_sys::Function::call1(&fun, &JsValue::NULL, &filled_props)))
                })
            }))
        })
    }).into_js_value()
}

#[wasm_bindgen(typescript_custom_section)]
const CONSTRUCT_COMPONENT: &'static str = r"
export function constructComponent<OptionalProps extends object, RequiredProps extends object>(
    component: Component<OptionalProps, RequiredProps>,
    key: string,
    props: Partial<OptionalProps> & RequiredProps,
)
";

#[wasm_bindgen(skip_typescript)]
pub fn create_component(component: js_sys::Object, key: js_sys::JsString, props: js_sys::Object) -> Result<JsValue, JsValue> {
    let component_fun: js_sys::Function = js_sys::Reflect::get(&component, &JsValue::from_str("_internal")).ok()
        .and_then(|x| x.dyn_into::<js_sys::Function>().ok())
        .map(Ok)
        .unwrap_or_else(|| Err(JsValue::from(&js_sys::Error::new("not a component"))))?;

    // TODO: Set context
    js_sys::Function::call2(&component_fun, &JsValue::NULL, &key, &props)
    // TODO: Pop context
}