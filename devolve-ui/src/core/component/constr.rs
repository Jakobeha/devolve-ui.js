//! Utilities to create terse constructors for your custom components,
//! since creating `VComponent`s manually is very verbose.

use crate::core::component::component::{VComponent, VComponentBody};
use crate::core::component::context::{VComponentContext, VComponentContextImpl};
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
    F: Fn(&mut VComponentContextImpl<'_, Props, ViewData>) -> VComponentBody<ViewData> + 'static
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

/// See `make_component2`, this one is required for macro expansion.
pub macro _make_component2(
    ($d:tt) @
    $vis:vis $name:ident,
    $fun:path,
    $Props:ident
) {
    /// Usage: `$name!(c, key, { settings: my_settings } vec![...] )`
    $vis macro $name(
        $d c:expr,
        $d key:expr,
        { $d ( $d field:ident : $d field_value:expr ),* }
        $d ( $d children:expr )?
    ) {
        make_component(
            $d c,
            $d key,
            $Props {
                $d ( $d field : $d field_value, )*
                $d ( children : $d children, )?
                ..Default::default()
            },
            $fun
        )
    }
}

/// Usage: `make_component2!(pub app, app_fn, AppProps)`
///
/// Like `make_component` except you must define the props and function yourself.
/// This one defines the macro.
///
/// Pro tip: To get IntelliJ to understand this macro you must also import `_make_component2`
pub macro make_component2(
    $vis:vis $name:ident,
    $fun:path,
    $Props:ident
) {
    _make_component2!(
        ($) @
        $vis $name,
        $fun,
        $Props
    );
}

/// Create a custom component. Creates a function and macro which you can call with the component's name.
///
/// Pro tip: To get IntelliJ to understand this macro you must also import `_make_component` and `_make_component2`
///
/// Usage:
///
/// ```
/// use devolve_ui::core::component::constr::{_make_component, _make_component2, make_component};
/// use devolve_ui::view_data::tui::constr::{vbox, text};
/// use devolve_ui::view_data::tui::tui::TuiViewData;
///
/// make_component!(pub basic, BasicProps {
///     text: String
/// }, {
///     text: Default::default()
/// }, <TuiViewData>|_c, text| {
///     vbox!({}, {}, vec![
///         text!({}, {}, "Hello world!")
///     ])
/// });
///
/// basic!(c, "basic", { text: "Hello world".into() })
/// ```
pub macro make_component(
    $vis:vis $name:ident,
    $Props:ident { $( $ty_field:ident : $ty_field_ty:ty ),* },
    { $( $default_field:ident : $default_field_default:expr ),* },
    <$ViewData:ty>|$c:ident $( , $field:ident)*| $body:expr
) {
    _make_component!(
        ($) @
        $vis $name,
        $Props { $( $ty_field : $ty_field_ty ),* },
        { $( $default_field : $default_field_default ),* },
        <$ViewData>|$c $( , $field)*| $body
    );
}

/// See `make_component`, this one is required for macro expansion.
pub macro _make_component(
    ($d:tt) @
    $vis:vis $name:ident,
    $Props:ident { $( $ty_field:ident : $ty_field_ty:ty ),* },
    { $( $default_field:ident : $default_field_default:expr ),* },
    <$ViewData:ty>|$c:ident $( , $field:ident)*| $body:expr
) {
    $vis struct $Props {
        $( pub $ty_field : $ty_field_ty ),*
    }

    impl Default for $Props {
        fn default() -> Self {
            Self {
                $( $default_field : $default_field_default ),*
            }
        }
    }

    fn $name($c: &mut Box<VComponent<$ViewData>>, props: &$Props) -> VComponentBody<$ViewData> {
        let $Props { $( $field ),* } = props;
        VComponentBody::new($body)
    }

    _make_component2!(
        ($d) @
        $vis $name,
        $name,
        $Props
    );
}

#[cfg(test)]
#[cfg(feature = "tui")]
mod tests {
    use crate::core::component::component::{VComponent, VComponentBody};
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

    fn my_component2_fn(_c: &mut Box<VComponent<TuiViewData>>, props: &MyComponent2Props) -> VComponentBody<TuiViewData> {
        VComponentBody::new(vbox!({}, {}, vec![
            text!({}, {}, "Hello world!".to_owned()),
            text!({}, {}, props.children.to_owned()),
        ]))
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