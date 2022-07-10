use js_sys;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const NODE_TYPE: &'static str = r"
type Node = any;
type Component<OptionalProps extends Object, Props extends OptionalProps> = (optionalProps: OptionalProps, requiredProps: Omit<Props, keyof OptionalProps>) => Node;
";

#[wasm_bindgen(typescript_custom_section)]
const MAKE_COMPONENT: &'static str = r"
export function make_component<OptionalProps extends object, Props extends OptionalProps>(
    fun: (props: Props) => Node,
    optional_prop_defaults: OptionalProps
): Component<OptionalProps, Props>;
";

#[wasm_bindgen(skip_typescript)]
pub fn make_component(fun: js_sys::Function, optional_prop_defaults: js_sys::Object) -> JsValue {
    Closure::new(move |optional_props: js_sys::Object, required_props: js_sys::Object| {
        let props = js_sys::Object::new();
        js_sys::Object::assign3(&props, &optional_prop_defaults, &optional_props, &required_props);
        // TODO: Set context
        js_sys::Function::call1(fun, &JsValue::NULL, &props)?;
        // TODO: Pop context
    }).into_js_value()
}

#[wasm_bindgen(typescript_custom_section)]
const CREATE_ELEMENT: &'static str = r"
export function createElement (
  element: undefined,
  props: {},
  ...children: VJSX[]
): VNode[];
export function createElement <Key extends keyof JSXIntrinsics> (
  element: Key,
  props: Omit<JSXIntrinsics[Key], 'children'>,
  ...children: IntoArray<JSXIntrinsics[Key]['children']>
): VView;
export function createElement <T extends VView, Props, Children extends any[]> (
  element: (props: Props & { children?: Children }) => T,
  props: Props & { key?: string },
  ...children: Children
): VComponent & { node: T };
";

#[wasm_bindgen(skip_typescript)]
pub fn create_element(element: JsValue, props: JsValue, children: JsValue) -> Result<JsValue, JsValue> {
    let props = match props.is_falsy() {
        false => props,
        true => js_sys::Object::new()
    };

    if element.is_undefined() {
        // Fragment

    } else if let Some(intrinsic) = element.as_string() {
        // Intrinsic
        let intrinsic_fn = intrinsic_fn(intrinsic);
        let args = js_sys::Array::new();
        js_sys::push(&args, &props);
        for child in children {
            js_sys::push(&args, &child);
        }
        js_sys::apply(intrinsic_fn, &JsValue::NULL, &props, &children)
    } else if let Some(make_component) = js_sys::Function::try_from(element) {
        // Component
        js_sys::define_property(&props, &JsValue::from_str("children"), &JsValue::from(children));
        js_sys::call1(make_component, &JsValue::NULL, &props)
    }
}