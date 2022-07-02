use js_sys;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const NODE_TYPE: &'static str = r"
type Node = any;
type Component<OptionalProps extends Object, Props extends OptionalProps> = (optionalProps: OptionalProps, requiredProps: Omit<Props, keyof OptionalProps>) => Node;
";

#[wasm_bindgen(typescript_custom_section)]
const MAKE_COMPONENT: &'static str = r"
declare function make_component<OptionalProps extends object, Props extends OptionalProps>(
    fun: (props: Props) => Node,
    optional_prop_defaults: OptionalProps
): Component<OptionalProps, Props>;
";

#[wasm_bindgen(skip_typescript)]
pub fn make_component(fun: js_sys::Function, optional_prop_defaults: js_sys::Object) -> JsValue {

}