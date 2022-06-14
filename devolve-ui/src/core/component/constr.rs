//! Utilities to create terse constructors for your custom components,
//! since creating `VComponent`s manually is very verbose.

use crate::core::component::component::{VComponent};
use crate::core::component::context::{VComponentContext, VComponentContext2};
use crate::core::component::node::VNode;
use crate::core::component::parent::VParent;
use crate::core::component::path::VComponentKey;
use crate::core::view::view::VViewData;

/// Crates the component and adds it to `c`.
/// The component can't be returned because it is in `c`.
/// A reference could be returned, but currently is not because there isn't any clear need for it;
/// submit an issue if you have a use case.
/// Instead a node is returned refernencing the component via `key`.
pub fn make_component<
    'a,
    ViewData: VViewData + 'static,
    Str: Into<VComponentKey>,
    Props: 'static,
    F: Fn(VComponentContext2<'_, Props, ViewData>) -> VNode<ViewData> + 'static
>(
    c: &'a mut impl VComponentContext<'a, ViewData=ViewData>,
    key: Str,
    props: Props,
    construct: F
) -> VNode<ViewData> {
    let parent = c.component();
    let component = VComponent::new(VParent::Component(parent), key.into(), props, construct);
    let component = parent.add_child(component);
    VNode::Component {
        id: component.head.id(),
        key: component.head.key()
    }
}

/// See `make_component_macro`, this one is required for macro expansion.
pub macro _make_component_macro(
    ($d:tt) @
    $vis:vis $name:ident,
    $fun:path,
    $Props:ident
) {
    /// Usage: `$name!(c, key, { optional_field: "optional value".to_string() }, "required value".to_string())`
    $vis macro $name(
        $d c:expr,
        $d key:expr,
        { $d ( $d opt_field:ident : $d opt_field_value:expr ),* }
        $d ( , $d req_prop_id:expr )*
    ) { {
        let props = $Props {
            $d ( $d opt_field : $d opt_field_value, )*
            ..$crate::core::misc::partial_default::PartialDefault::partial_default(($d ( $d req_prop_id, )*))
        };
        make_component(
            $d c,
            $d key,
            props,
            $fun
        )
    } }
}

/// Usage:
/// ```rust
/// use devolve_ui::core::component::constr::make_component_macro;
/// use devolve_ui::core::component::context::VComponentContext2;
/// use devolve_ui::core::component::node::VNode;
/// use devolve_ui::core::misc::partial_default::PartialDefault;
/// use devolve_ui::view_data::tui::constr::text;
/// use devolve_ui::view_data::tui::tui::TuiViewData;
///
/// pub struct AppProps {
///     required_field1: String,
///     required_field2: String
/// }
///
/// impl PartialDefault for AppProps {
///     type RequiredArgs = (String, String,);
///
///     fn partial_default((required_field1, required_field2,): Self::RequiredArgs) -> Self {
///         Self {
///             required_field1,
///             required_field2
///         }
///     }
/// }
///
/// fn app((c, AppProps { required_field1, required_field2 }): &mut VComponentContext2<AppProps, TuiViewData>) -> VNode<TuiViewData> {
///     text!({}, {}, "Hello world!".to_string())
/// }
///
/// fn app_fn_with_weird_name((c, AppProps { required_field1, required_field2 }): &mut VComponentContext2<AppProps, TuiViewData>) -> VNode<TuiViewData> {
///     text!({}, {}, "Hello world!".to_string())
/// }
///
/// make_component_macro!(pub app, AppProps [ required_field1 required_field2 ]);
/// // or
/// make_component_macro!(pub app2, app_fn_with_weird_name, AppProps [ required_field1, required_field2 ]);
/// ```
///
/// Like `make_component` except you must define the props and defaults yourself.
/// This one only defines the macro.
///
/// Pro tip: To get IntelliJ to understand this macro you must also import `_make_component_macro`
pub macro make_component_macro {
    ($vis:vis $name:ident, $Props:ident) => {
        _make_component_macro!(
            ($) @
            $vis $name,
            $name,
            $Props
        );
    },
    ($vis:vis $name:ident, $fun:path, $Props:ident) => {
        _make_component_macro!(
            ($) @
            $vis $name,
            $fun,
            $Props
        );
    },
}

/// Create a custom component. Creates a function and macro which you can call with the component's name.
///
/// Pro tip: To get IntelliJ to understand this macro you must also import `_make_component_macro`
///
/// Usage:
///
/// ```
/// use devolve_ui::core::component::constr::{_make_component_macro, make_component};
/// use devolve_ui::core::component::context::VComponentContext2;
/// use devolve_ui::core::component::node::VNode;
/// use devolve_ui::view_data::tui::constr::{vbox, text};
/// use devolve_ui::view_data::tui::tui::TuiViewData;
///
/// make_component!(pub basic, BasicProps {
///     optional_field: String = "default value".to_string(),
///     another_optional: usize = 1
/// } [ required_field: String ]);
///
/// fn basic((c, BasicProps { optional_field, another_optional, required_field }): VComponentContext2<BasicProps, TuiViewData>) -> VNode<TuiViewData> {
///     vbox!({}, {}, vec![
///         text!({}, {}, format!("{} and {}", requireD_field, optional_field)),
///         text!({}, {}, "Hello world!".to_string())
///     ])
/// }
///
/// fn somewhere_else((c, ()): VComponentContext2<(), TuiViewData>) {
///     basic!(&mut c, "key", {
///         optional_field: "overridden value".to_string()
///     }, "required value".to_string())
/// }
/// ```
pub macro make_component(
    $vis:vis $name:ident,
    $Props:ident
    { $( $opt_prop_id:ident : $opt_prop_ty:ty = $opt_prop_default:expr ),* }
    [ $( $req_prop_id:ident : $req_prop_ty:ty ),* ]
) {
    $vis struct $Props {
        $( pub $opt_prop_id : $opt_prop_ty, )*
        $( pub $req_prop_id : $req_prop_ty, )*
    }

    impl $crate::core::misc::partial_default::PartialDefault for $Props {
        type RequiredArgs = ( $( $req_prop_ty, )* );

        fn partial_default(($( $req_prop_id, )*): Self::RequiredArgs) -> Self {
            Self {
                $( $opt_prop_id : $opt_prop_default, )*
                $( $req_prop_id, )*
            }
        }
    }

    _make_component_macro!(
        ($) @
        $vis $name,
        $name,
        $Props
    );
}

#[cfg(test)]
#[cfg(feature = "tui")]
mod tests {
    use crate::core::component::component::VComponent;
    use crate::core::component::constr::{make_component, make_component2};
    use crate::core::component::node::VNode;
    use crate::core::renderer::renderer::Renderer;
    use crate::core::view::layout::macros::smt;
    use crate::engines::tui::tui::{TuiConfig, TuiEngine};
    use crate::view_data::tui::constr::{text, vbox};
    use crate::view_data::tui::tui::TuiViewData;

    #[derive(Default)]
    struct MyComponent2Props {
        pub children: &'static str,
        #[allow(dead_code)]
        pub settings: &'static str,
    }

    fn my_component2_fn(_c: &mut Box<VComponent<TuiViewData>>, props: &MyComponent2Props) -> VNode<TuiViewData> {
        vbox!({}, {}, vec![
            text!({}, {}, "Hello world!".to_owned()),
            text!({}, {}, props.children.to_owned()),
        ])
    }

    make_component2!(pub my_component2, my_component2_fn, MyComponent2Props);

    #[test]
    fn test_component2() {
        let renderer = Renderer::new(TuiEngine::new(TuiConfig::default()));
        renderer.root(|c| my_component2!(c, "key", { settings: "Override settings" } "Override text"));
    }

    make_component!(pub my_component, MyComponentProps {
        title: String,
        children: Vec<VNode<TuiViewData>>
    }, {
        title: String::from("Untitled"),
        children: Vec::new()
    }, <TuiViewData>|_c, title, children| {
        vbox!({ width: smt!(100%) }, {}, vec![
            text!({}, {}, title.clone()),
            text!({}, {}, children.len().to_string()),
        ])
    });

    #[test]
    fn test_component() {
        let renderer = Renderer::new(TuiEngine::new(TuiConfig::default()));
        renderer.root(|c| my_component!(c, "key", { title: "Override title".to_owned() } vec![
            text!({}, {}, "Hello world!".to_owned()),
        ]));
    }
}