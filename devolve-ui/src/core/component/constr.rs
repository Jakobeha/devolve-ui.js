use crate::core::component::component::{VComponent, VComponentBody, VComponentKey};
use crate::core::component::node::VNode;
use crate::core::view::view::VViewData;

pub fn make_component<
    ViewData: VViewData + 'static,
    Str: Into<VComponentKey>,
    Props: 'static,
    F: Fn(&mut Box<VComponent<ViewData>>, &Props) -> VComponentBody<ViewData> + 'static
>(
    c: &mut Box<VComponent<ViewData>>,
    key: Str,
    props: Props,
    construct: F
) -> VNode<ViewData> {
    VNode::Component(VComponent::new(c.into(), &key.into(), props, construct))
}

/// Usage: `make_component2!(pub app, app_fn, AppProps, { } + default)`
macro _make_component2(
    ($d:tt) @
    $vis:vis $name:ident,
    $fun:path,
    $Props:ident
) {
    /// Usage: `$name!(c, "key", { settings: my_settings } [...] )`
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

/// Usage: `make_component2!(pub app, app_fn, AppProps, { } + default)`
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

/// Usage:
///
/// ```
/// use devolve_ui::core::component::constr::make_component;
///
/// make_component!(pub app, { ...app props }, { ...app prop defaults }, |c, ...prop names| {
///     ...app body
/// })
/// ```
pub macro make_component(
    $vis:vis $name:ident,
    $Props:ident { $( $ty_field:ident : $ty_field_type:ty ),* },
    { $( $default_field:ident : $default_field_default:expr ),* },
    <$ViewData:ty>|$c:ident $( , $field:ident)*| $body:expr
) {
    $vis struct $Props {
        $( pub $ty_field : $ty_field_type ),*
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
        ($) @
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