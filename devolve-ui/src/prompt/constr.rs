//! Create component constructors for your custom prompts,

/// See `make_prompt_macro`, this one is required for macro expansion.
pub macro _make_prompt_macro(
    ($d:tt) @
    $vis:vis $name:ident,
    $fun:path,
    $Props:ident
) {
    /// To create component: `$name!(c, key, { optional_field: "optional value".to_string() }, "required value".to_string())`
    ///
    /// To create prompt: `$name![c, { optional_field: "optional value".to_string() }, "required value".to_string()]`
    ///
    /// If there are prompt props: `$name!(c, key, [ prompt_field1, prompt_field2, ...], { optional_field: "optional value".to_string() }, "required value".to_string())`
    $vis macro $name {
        (
            $d c:expr,
            $d key:expr,
            $d ( [ $d ( $d prompt_field:expr ),* $d ( , )? ] )?
            { $d ( $d opt_field:ident : $d opt_field_value:expr ),* $d ( , )? }
            $d ( , $d req_prop_id:expr )*
        ) => { {
            let props = $Props {
                $d ( $d opt_field : $d opt_field_value, )*
                ..$crate::core::misc::partial_default::PartialDefault::partial_default(($d ( $d req_prop_id, )*))
            };
            $crate::core::component::constr::make_component(
                &mut $d c,
                $d key,
                props,
                $crate::prompt::prompt_fn_into_component_fn($fun, move || (
                    $d ( $d ( $d prompt_field ),* )?
                ))
            )
        } },
        [
            $d c:expr,
            $d ( [ $d ( $d prompt_field:expr ),* $d ( , )? ] )?
            { $d ( $d opt_field:ident : $d opt_field_value:expr ),* $d ( , )? }
            $d ( , $d req_prop_id:expr )*
        ] => { {
            let props = $Props {
                $d ( $d opt_field : $d opt_field_value, )*
                ..$crate::core::misc::partial_default::PartialDefault::partial_default(($d ( $d req_prop_id, )*))
            };
            $fun((c, props, move || (
                $d ( $d ( $d prompt_field ),* )?
            )))
        } }
    }
}

/// Usage:
/// ```rust
/// use devolve_ui::prompt::constr::make_prompt_macro;
/// use devolve_ui::prompt::context::VPromptContext2;
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
/// async fn app((c, ()): &mut VPromptContext2<AppProps, TuiViewData, ()>) {
///     c.yield_void(|(c, resume, AppProps { required_field1, required_field2 })| {
///         text!({}, {}, "Hello world!".to_string())
///     }).await;
/// }
///
/// async fn app_fn_with_weird_name((c, AppProps { required_field1, required_field2 }): &mut VPromptContext2<AppProps, TuiViewData, ()>) {
///     c.yield_void(|(c, resume, AppProps { required_field1, required_field2 })| {
///         text!({}, {}, "Hello world!".to_string())
///     }).await;
/// }
///
/// make_prompt_macro!(pub app, AppProps [ required_field1 required_field2 ]);
/// // or
/// make_prompt_macro!(pub app2, app_fn_with_weird_name, AppProps [ required_field1, required_field2 ]);
/// ```
///
/// Like `make_prompt` except you must define the props and defaults yourself.
/// This one only defines the macro.
///
/// Pro tip: To get IntelliJ to understand this macro you must also import `_make_prompt_macro`
pub macro make_prompt_macro {
    ($vis:vis $name:ident, $Props:ident) => {
        _make_prompt_macro!(
            ($) @
            $vis $name,
            $name,
            $Props
        );
    },
    ($vis:vis $name:ident, $fun:path, $Props:ident) => {
        _make_prompt_macro!(
            ($) @
            $vis $name,
            $fun,
            $Props
        );
    },
}

/// Create a custom component. Creates a function and macro which you can call with the component's name.
///
/// Pro tip: To get IntelliJ to understand this macro you must also import `_make_prompt_macro`
///
/// Usage:
///
/// ```
/// use std::any::Any;
/// use devolve_ui::prompt::constr::{_make_prompt_macro, make_prompt};
/// use devolve_ui::prompt::context::VPromptContext2;
/// use devolve_ui::core::component::node::VNode;
/// use devolve_ui::core::misc::shorthand::d;
/// use devolve_ui::view_data::tui::constr::{vbox, text};
/// use devolve_ui::view_data::tui::tui::{HasTuiViewData, TuiViewData};
///
/// // Define
///
/// make_prompt!(pub basic, Basic<TParam> where (TParam: Any) {
///     optional_field: String = "default value".to_string(),
///     another_optional: usize = 1
/// } [ required_field: String ]);
///
/// async fn basic<TParam: Any, ViewData: HasTuiViewData>((c, ()): VPromptContext2<Basic<TParam>, ViewData, ()>) {
///     c.yield_void(|(c, resume, Basic { optional_field, another_optional, required_field })| {
///         vbox(d(), d(), vec![
///             text!({}, {}, format!("{} and {}", required_field, optional_field)),
///             text!({}, {}, "Hello world!".to_string())
///         ])
///    }).await;
/// }
///
/// // Use
///
/// async fn pass_this_to_renderer_construct<ViewData: HasTuiViewData>((mut c, ()): VPromptContext2<(), ViewData, ()>) {
///     c.yield_void(|(c, resume, ())| {
///         basic!((), {
///             optional_field: "overridden value".to_string(),
///         }, "required value".to_string())
///    }).await;
/// }
/// ```
pub macro make_prompt(
    $vis:vis $name:ident,
    $Props:ident $( < $( $T:ident $( : $TTy:tt $( + $TTyRest:tt )* )? ),* > )? $( where ( $( $more_bounds:tt )* ) )?
    { $( $opt_prop_id:ident : $opt_prop_ty:ty = $opt_prop_default:expr ),* $( , )? }
    [ $( $req_prop_id:ident : $req_prop_ty:ty ),* $( , )? ]
) {
    $vis struct $Props $( < $( $T $( : $TTy $( + $TTyRest )* )? ),* > )? $( where $( $more_bounds )* )? {
        $( pub $opt_prop_id : $opt_prop_ty, )*
        $( pub $req_prop_id : $req_prop_ty, )*
    }

    impl $( < $( $T $( : $TTy $( + $TTyRest )* )? ),* > )? $crate::core::misc::partial_default::PartialDefault for $Props $( < $( $T ),* > )? $( where $( $more_bounds )* )? {
        type RequiredArgs = ( $( $req_prop_ty, )* );

        fn partial_default(($( $req_prop_id, )*): Self::RequiredArgs) -> Self {
            Self {
                $( $opt_prop_id : $opt_prop_default, )*
                $( $req_prop_id, )*
            }
        }
    }

    _make_prompt_macro!(
        ($) @
        $vis $name,
        $name,
        $Props
    );
}

#[cfg(test)]
#[cfg(feature = "tui")]
mod tests {
    use crate::prompt::context::VPromptContext2;
    #[allow(unused_imports)]
    use crate::prompt::constr::{_make_prompt_macro, make_prompt, make_prompt_macro};
    use crate::core::component::node::VNode;
    use crate::core::renderer::renderer::Renderer;
    use crate::core::view::layout::macros::smt;
    use crate::engines::tui::tui::{TuiConfig, TuiEngine};
    use crate::view_data::tui::constr::{vbox, text};
    use crate::view_data::tui::tui::HasTuiViewData;

    #[derive(Default)]
    struct MyComponent2Props {
        pub text: &'static str,
        #[allow(dead_code)]
        pub settings: &'static str,
    }

    async fn my_component2_fn<ViewData: HasTuiViewData>((mut c, ()): VPromptContext2<MyComponent2Props, ViewData, ()>) {
        c.yield_void(|(_c, _resume, MyComponent2Props { settings: _settings, text })| {
            vbox!({}, {}, vec![
                text!({}, {}, "Hello world!".to_string()),
                text!({}, {}, text.to_string()),
            ])
        }).await;
    }

    make_prompt_macro!(pub my_component2, my_component2_fn, MyComponent2Props);

    #[test]
    fn test_component2() {
        let renderer = Renderer::new(TuiEngine::new(TuiConfig::default()));
        renderer.root(|(mut c, ())| my_component2!(c, "key", { text: "Override text" }));
    }

    make_prompt!(pub my_component, MyComponentProps<ViewData: HasTuiViewData + Clone> {
        title: String = String::from("Untitled")
    } [children: Vec<VNode<ViewData>>]);

    async fn my_component<ViewData: HasTuiViewData + Clone + 'static>((mut c, ()): VPromptContext2<MyComponentProps<ViewData>, ViewData, ()>) {
        c.yield_void(|(_c, _resume, MyComponentProps { title, children })| {
            vbox!({ width: smt!(100%) }, {}, vec![
                text!({}, {}, title.clone()),
                vbox!({}, {}, children.clone())
            ])
        }).await;
    }

    #[test]
    fn test_component() {
        let renderer = Renderer::new(TuiEngine::new(TuiConfig::default()));
        renderer.root(|(mut c, ())| my_component!(c, "key", { title: "Override title".to_owned() }, vec![
            text!({}, {}, "Hello world!".to_owned()),
        ]));
    }
}