// We explicitly provide TypeScript declarations for all our bindings
// #![wasm_bindgen(skip_typescript)]

use js_sys;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write;
use arrayvec::{ArrayString, CapacityError};
use devolve_ui::component::constr::make_component;
use devolve_ui::component::context::{VComponentContext1, VComponentContextUnsafe};
use devolve_ui::component::node::{NodeId, VNode};
use devolve_ui::component::path::VComponentKey;
use devolve_ui::view::dyn_view_data::DynViewData;

pub type JsViewData = DynViewData;
pub type JsNode = VNode<JsViewData>;
pub type JsComponentContext<'a, 'a0> = VComponentContext1<'a, 'a0, js_sys::Object, JsViewData>;

pub struct JsComponentKey<'a>(&'a js_sys::JsString);

impl<'a> TryInto<VComponentKey> for JsComponentKey<'a> {
    type Error = CapacityError<()>;

    fn try_into(self) -> Result<VComponentKey, Self::Error> {
        let mut array_str = ArrayString::<{ VComponentKey::SIZE }>::new();
        write!(array_str, "{}", self.0).map_err(|_err| CapacityError::new(()))?;
        Ok(VComponentKey::new(array_str))
    }
}

thread_local! {
    static CONTEXTS: RefCell<Vec<VComponentContextUnsafe>> = RefCell::new(Vec::new());
    static NODES: RefCell<HashMap<NodeId, JsNode>> = RefCell::new(HashMap::new());
}

#[derive(Debug)]
struct NoTopContextError;

#[derive(Debug)]
struct MissingNodeError;

#[derive(Debug)]
enum JsToVNodeError {
    InvalidArgument,
    Occupied
}

impl Into<js_sys::Error> for JsToVNodeError {
    fn into(self) -> js_sys::Error {
        match self {
            JsToVNodeError::InvalidArgument => js_sys::Error::new("invalid argument"),
            JsToVNodeError::Occupied => js_sys::Error::new("node id already occupied")
        }
    }
}

impl Into<js_sys::Error> for VNodeToJsError {
    fn into(self) -> js_sys::Error {
        match self {
            VNodeToJsError::Occupied => js_sys::Error::new("node id already occupied")
        }
    }
}

impl Into<js_sys::Error> for MissingNodeError {
    fn into(self) -> js_sys::Error {
        js_sys::Error::new("node not found")
    }
}

impl Into<js_sys::Error> for NoTopContextError {
    fn into(self) -> js_sys::Error {
        js_sys::Error::new("no top context")
    }
}

#[derive(Debug)]
enum VNodeToJsError {
    Occupied
}

fn with_top_context<R, F: FnOnce(JsComponentContext) -> R>(fun: F) -> Result<R, NoTopContextError> {
    CONTEXTS.with_borrow(|contexts| {
        let context = *contexts.last().ok_or(NoTopContextError)?;
        let context = unsafe { VComponentContext1::from_unsafe(context) };
        Ok(fun(context))
    })
}

fn with_push_context<R, F: FnOnce() -> R>(context: &mut JsComponentContext, fun: F) -> R {
    CONTEXTS.with_borrow_mut(|contexts| contexts.push(context.as_unsafe()));
    let result = fun();
    CONTEXTS.with_borrow_mut(|contexts| contexts.pop().unwrap());
    result
}

fn vnode_to_js(node: VNode<DynViewData>) -> Result<JsValue, VNodeToJsError> {
    let node_id = node.id();
    NODES.with_borrow_mut(|nodes| nodes.try_insert(node_id, node).map(|_ref| ()).map_err(|_err| VNodeToJsError::Occupied))?;
    Ok(JsValue::from(node_id.into_usize()))
}

fn js_to_vnode(js: JsValue) -> Result<VNode<DynViewData>, JsToVNodeError> {
    let node_id: f64 = js.as_f64().ok_or(JsToVNodeError::InvalidArgument)?;
    if node_id.floor() != node_id {
        return Err(JsToVNodeError::InvalidArgument);
    }
    let node_id = NodeId::from_usize(node_id as usize);

    NODES.with_borrow_mut(|nodes| nodes.remove(&node_id).ok_or(JsToVNodeError::Occupied))
}

fn collapse_vnode(vnode: Result<VNode<DynViewData>, JsValue>) -> VNode<DynViewData> {
    match vnode {
        Err(err) => todo!("{:?}", err),
        Ok(node) => node
    }
}

fn error_to_js_value(err: impl Into<js_sys::Error>) -> JsValue {
    JsValue::from(err.into())
}

#[wasm_bindgen(skip_typescript)]
pub fn define_component(fun: js_sys::Function, optional_prop_defaults: js_sys::Object) -> JsValue {
    Closure::<dyn Fn(js_sys::JsString, js_sys::Object) -> Result<JsValue, JsValue>>::new(move |key: js_sys::JsString, props: js_sys::Object| {
        let fun = fun.clone();
        let filled_props = js_sys::Object::new();
        js_sys::Object::assign2(&filled_props, &optional_prop_defaults, &props);
        with_top_context(move |mut c| {
            vnode_to_js(make_component(&mut c, JsComponentKey(&key), filled_props, move |(mut c, filled_props)| {
                let fun = fun.clone();
                with_push_context(&mut c, move || {
                    collapse_vnode(js_sys::Function::call1(&fun, &JsValue::NULL, &filled_props).and_then(|ret| js_to_vnode(ret).map_err(error_to_js_value)))
                })
            }))
        }).map(|result| result.map_err(error_to_js_value)).map_err(error_to_js_value).flatten()
    }).into_js_value()
}

pub fn create_component(component: js_sys::Object, key: js_sys::JsString, props: js_sys::Object) -> Result<JsValue, JsValue> {
    let component_fun: js_sys::Function = js_sys::Reflect::get(&component, &JsValue::from_str("_internal")).ok()
        .and_then(|x| x.dyn_into::<js_sys::Function>().ok())
        .map(Ok)
        .unwrap_or_else(|| Err(JsValue::from(&js_sys::Error::new("not a component"))))?;

    // TODO: Set context
    js_sys::Function::call2(&component_fun, &JsValue::NULL, &key, &props)
    // TODO: Pop context
}