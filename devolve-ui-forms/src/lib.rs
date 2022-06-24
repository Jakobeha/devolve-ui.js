#![feature(decl_macro)]

use std::any::Any;
use std::borrow::Cow;
use std::collections::BTreeSet;
use std::marker::PhantomData;
use devolve_ui::core::component::constr::{_make_component_macro, make_component};
use devolve_ui::core::component::context::{VComponentContext1, VComponentContext2, VEffectContext1, VEffectContext2};
use devolve_ui::core::component::node::VNode;
use devolve_ui::core::hooks::context::{ContextIdSource, use_consume, use_provide};
use devolve_ui::core::hooks::event::use_key_listener_when;
use devolve_ui::core::hooks::state::use_state;
use devolve_ui::core::misc::shorthand::d;
use devolve_ui::core::renderer::input::KeyCode;
use devolve_ui::core::view::layout::macros::{mt, smt};
use devolve_ui::core::view::view::VViewData;
use devolve_ui::view_data::attrs::BorderStyle;
use devolve_ui::view_data::tui::constr::*;
use devolve_ui::view_data::tui::tui::HasTuiViewData;

make_component!(pub focus_provider, FocusProviderProps<ViewData: VViewData> {
    enable_tab: bool = false,
} [content: VNode<ViewData>]);

make_component!(pub text_field, TextFieldProps<Props: Any, ViewData: VViewData> {
    initial_value: Cow<'static, str> = "".into(),
    placeholder: Cow<'static, str> = "".into(),
    is_enabled: bool = true,
    override_value: Option<String> = None,
    on_change: Option<Box<dyn Fn(VEffectContext2<Props, ViewData>, &str)>> = None,
    phantom: PhantomData<(Props, ViewData)> = PhantomData
} []);

#[derive(Default)]
pub struct FocusContext {
    pub focusable_ids: BTreeSet<usize>,
    pub next_free_id: usize,
    pub focused_id: Option<usize>
}

pub trait LocalFocus {
    type Props: Any;
    type ViewData: VViewData;

    fn is_focused(&self, c: &mut VEffectContext1<Self::Props, Self::ViewData>) -> bool;
    fn focus(&mut self, c: &mut VEffectContext1<Self::Props, Self::ViewData>);
}

struct LocalFocusImpl<Props: Any, ViewData: VViewData, F1: Fn(&VEffectContext1<Props, ViewData>) -> bool, F2: FnMut(&mut VEffectContext1<Props, ViewData>)> {
    is_focused: F1,
    focus: F2,
    phantom: PhantomData<(Props, ViewData)>
}

impl <Props: Any, ViewData: VViewData, F1: Fn(&VEffectContext1<Props, ViewData>) -> bool, F2: FnMut(&mut VEffectContext1<Props, ViewData>)> LocalFocus for LocalFocusImpl<Props, ViewData, F1, F2> {
    type Props = Props;
    type ViewData = ViewData;

    fn is_focused(&self, c: &mut VEffectContext1<Self::Props, Self::ViewData>) -> bool {
        (self.is_focused)(c)
    }

    fn focus(&mut self, c: &mut VEffectContext1<Self::Props, Self::ViewData>) {
        (self.focus)(c)
    }
}

static FOCUS_PROVIDER_CONTEXT: ContextIdSource<FocusContext> = ContextIdSource::new();

pub fn focus_provider<ViewData: VViewData + Clone + 'static>((mut c, FocusProviderProps {
    content,
    enable_tab
}): VComponentContext2<FocusProviderProps<ViewData>, ViewData>) -> VNode<ViewData> {
    let focus_context = use_provide(&mut c, &FOCUS_PROVIDER_CONTEXT, || Box::new(FocusContext::default()));

    use_key_listener_when(&mut c, *enable_tab, move |(mut c, FocusProviderProps { content, enable_tab }), event| {
        match event.code {
            KeyCode::Tab => {
                let focus_context = focus_context.get_mut(&mut c);
                if let Some(id) = focus_context.focused_id {
                    focus_context.focused_id = focus_context.focusable_ids.iter().skip_while(|&&id2| id2 <= id).next().copied();
                }
                if focus_context.focused_id.is_none() {
                    focus_context.focused_id = focus_context.focusable_ids.iter().next().copied();
                }
            }
            KeyCode::BackTab => {
                let focus_context = focus_context.get_mut(&mut c);
                if let Some(id) = focus_context.focused_id {
                    focus_context.focused_id = focus_context.focusable_ids.iter().rev().skip_while(|&&id2| id2 >= id).next().copied();
                }
                if focus_context.focused_id.is_none() {
                    focus_context.focused_id = focus_context.focusable_ids.iter().next_back().copied();
                }
            }
            _ => {}
        }
    });

    content.clone()
}

pub fn use_focus<Props: Any, ViewData: VViewData + 'static>(c: &mut VComponentContext1<Props, ViewData>) -> Box<dyn LocalFocus<Props=Props, ViewData=ViewData>> {
    let focus_context = use_consume(c, &FOCUS_PROVIDER_CONTEXT);
    let my_id = focus_context.get(c).next_free_id;
    focus_context.get_mut(c).next_free_id += 1;
    focus_context.get_mut(c).focusable_ids.insert(my_id);

    // TODO: Make LocalFocusImpl store focus_context and my_id, then make it not a Box<dyn>
    Box::new(LocalFocusImpl {
        is_focused: move |c| focus_context.get(c).focused_id == Some(my_id),
        focus: move |mut c| focus_context.get_mut(c).focused_id = Some(my_id),
        phantom: PhantomData
    })
}

pub fn text_field<Props: Any, ViewData: HasTuiViewData + 'static>((mut c, TextFieldProps { initial_value, placeholder, is_enabled, override_value, on_change, phantom }): VComponentContext2<TextFieldProps<Props, ViewData>, ViewData>) -> VNode<ViewData> {
    let mut focus = use_focus(&mut c);
    let mut value = use_state(&mut c, || initial_value.to_string());

    use_key_listener_when(&mut c, *is_enabled, |mut c, key| {
        todo!("change text on key events");
    });

    let txt = format!("{}█", override_value.as_ref().unwrap_or_else(|| value.get(&c)));

    zbox(Vi1 {
        width: smt!(16 u),
        ..d()
    }, d(), vec![
        text(Vi1 {
            x: mt!(1 u),
            y: mt!(1 u),
            width: smt!(prev - 2 u),
            height: smt!(1 u),
            ..d()
        }, d(), txt),
        border(Vi1 {
            width: smt!(100 %),
            height: smt!(100 %),
            ..d()
        }, d(), BorderStyle::Rounded)
    ])
}

#[cfg(test)]
mod test {
    use std::io;
    use devolve_ui::core::component::constr::{_make_component_macro, make_component};
    use devolve_ui::core::component::context::{VComponentContext2, VEffectContext2};
    use devolve_ui::core::component::node::VNode;
    use devolve_ui::core::misc::shorthand::d;
    use devolve_ui::core::renderer::renderer::Renderer;
    use devolve_ui::core::view::layout::macros::{mt, smt};
    use devolve_ui::engines::tui::tui::{TuiConfig, TuiEngine};
    use devolve_ui::view_data::tui::constr::{VBo1, vbox, Vi1, zbox};
    #[cfg(feature = "tui-images")]
    use devolve_ui::view_data::tui::terminal_image::TuiImageFormat;
    use devolve_ui::view_data::tui::tui::HasTuiViewData;
    use crate::{focus_provider, FocusProviderProps, text_field};

    make_component!(test_app, TestAppProps {} []);

    fn test_app<ViewData: HasTuiViewData + Clone + 'static>((mut c, TestAppProps {}): VComponentContext2<TestAppProps, ViewData>) -> VNode<ViewData> {
        zbox(Vi1 {
            width: smt!(100 %),
            height: smt!(100 %),
            ..d()
        }, d(), vec![
            focus_provider!(&mut c, (), {}, vbox(Vi1 {
                x: mt!(2 u),
                y: mt!(2 u),
                width: smt!(100 % - 4 u),
                height: smt!(100 % - 4 u),
                ..d()
            }, VBo1 {
                gap: mt!(1 u),
                ..d()
            }, vec![
                text_field!(&mut c, 1, {
                    initial_value: "".into(),
                    placeholder: "field 1".into(),
                    is_enabled: true,
                    override_value: None,
                    on_change: None as Option<Box<dyn Fn(VEffectContext2<TestAppProps, ViewData>, &str)>>
                }),
                text_field!(&mut c, 2, {
                    initial_value: "field 2".into(),
                    placeholder: "field 2".into(),
                    is_enabled: true,
                    override_value: None,
                    on_change: None as Option<Box<dyn Fn(VEffectContext2<TestAppProps, ViewData>, &str)>>
                }),
                text_field!(&mut c, 3, {
                    initial_value: "".into(),
                    placeholder: "field 3".into(),
                    is_enabled: true,
                    override_value: None,
                    on_change: None as Option<Box<dyn Fn(VEffectContext2<TestAppProps, ViewData>, &str)>>
                }),
                text_field!(&mut c, 4, {
                    initial_value: "".into(),
                    placeholder: "field 4".into(),
                    is_enabled: false,
                    override_value: Some("override".into()),
                    on_change: None as Option<Box<dyn Fn(VEffectContext2<TestAppProps, ViewData>, &str)>>
                })
            ]))
        ])
    }

    #[test]
    pub fn test() {
        let renderer = Renderer::new(TuiEngine::new(TuiConfig {
            input: io::stdin(),
            output: io::stdout(),
            raw_mode: true,
            #[cfg(target_family = "unix")]
            termios_fd: None,
            #[cfg(feature = "tui-images")]
            image_format: TuiImageFormat::FallbackColor
        }));
        renderer.root(|(mut c, ())| test_app!(&mut c, (), {}));
        renderer.resume_blocking();
    }
}
