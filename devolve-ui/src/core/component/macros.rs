/// Usage: `make_component!(app!, app_fn, AppProps, { ..Default::default() })`
pub macro make_component(
    $vis:vis macro $name:ident,
    $fun:path,
    $Props:ident
    $( , { $( $field:ident : $field_value:expr ),* $( , $( ..$Default_or_Props:ident::default() )? )? } )?
    $( , { ..$Default_or_Props2:ident::default() } )?
) {
    /// Usage: `$name!(c, "key", { settings: my_settings } [...] )`
    $vis macro $name(
        $$c:expr,
        $$key:expr,
        $$( { $$( $$field:ident : $$field_value:expr ),* $$( , )? } )?
        $$( $$text:literal )?
        $$( [ $$( $$child:expr ),* $$( , )? ] )?
    ) {
        // Create props
        let props = dedup_struct_fields!($Props {
            $$( $$( $$field : $$field_value, )* )?
            $$( text : $$text, )?
            $$( children: vec![ $$( $$child ),* ], )?
            $( $( $field : $field_value, )* $( $( ..$Default_or_Props.default() )? )? )? $( $( ..$Default_or_Props2.default() )? )?
        });

        $crate::core::component::node::VNode::Component($crate::core::component::component::VComponent::new(
            $crate::core::component::parent::VParent::Component($$c),
            &$$key.into(),
            &props,
            $fun
        ))
    }
}

#[cfg(test)]
#[cfg(feature = "tui")]
mod tests {
    use crate::core::component::component::VComponent;
    use crate::core::component::macros::make_component;
    use crate::core::component::node::VNode;
    use crate::core::view::layout::bounds::{LayoutPosition1D, Measurement};
    use crate::view_data::tui::tui::TuiViewData;

    #[derive(Default)]
    struct MyComponentProps {
        text: &'static str,
        children: Vec<VNode<TuiViewData>>,
        settings: &'static str,
    }

    fn my_component_fn(c: &mut Box<VComponent<TuiViewData>>, props: &MyComponentProps) -> VNode<TuiViewData> {
        vbox!([
            text!("Hello world!"),
            text!(props.text),
        ])
    }

    make_component!(macro my_component, my_component_fn, MyComponentProps, {
        text: "Default text",
        ..Default::default()
    });

    #[test]
    fn text_component() {
        my_component!(c, "key", { settings: "Override settings" } "Override text");
    }
}