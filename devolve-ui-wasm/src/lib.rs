use js_sys;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

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

#[wasm_bindgen(skip_typescript)]
pub fn define_component(fun: js_sys::Function, optional_prop_defaults: js_sys::Object) -> JsValue {
    Closure::<dyn Fn(js_sys::JsString, js_sys::Object) -> Result<JsValue, JsValue>>::new(move |key: js_sys::JsString, props: js_sys::Object| {
        let filled_props = js_sys::Object::new();
        js_sys::Object::assign2(&filled_props, &optional_prop_defaults, &props);
        // TODO: Set context
        let result = js_sys::Function::call1(&fun, &JsValue::NULL, &filled_props);
        // TODO: Pop context (even if err)
        result
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