#![feature(decl_macro)]

use devolve_ui::component::context::VComponentContext2;
#[allow(unused_imports)]
use devolve_ui::component::constr::{_make_component_macro, make_component, make_component_macro};
use devolve_ui::component::node::VNode;
use devolve_ui::renderer::renderer::Renderer;
use devolve_ui::view::layout::macros::smt;
use devolve_ui_tui::engine::tui::{TuiConfig, TuiEngine};
use devolve_ui_tui::view_data::constr::{vbox, text};
use devolve_ui_tui::view_data::tui::HasTuiViewData;

#[derive(Default)]
struct MyComponent2Props {
    pub text: &'static str,
    #[allow(dead_code)]
    pub settings: &'static str,
}

fn my_component2_fn<ViewData: HasTuiViewData>((_c, MyComponent2Props { settings: _settings, text }): VComponentContext2<MyComponent2Props, ViewData>) -> VNode<ViewData> {
    vbox!({}, {}, vec![
            text!({}, {}, "Hello world!".to_string()),
            text!({}, {}, text.to_string()),
        ])
}

make_component_macro!(pub my_component2, my_component2_fn, MyComponent2Props);

#[test]
fn test_component2() {
    let renderer = Renderer::new(TuiEngine::new(TuiConfig::default()));
    renderer.root(|(mut c, ())| my_component2!(c, "key", { text: "Override text" }));
}

make_component!(pub my_component, MyComponentProps<ViewData: HasTuiViewData + Clone> {
    title: String = String::from("Untitled")
} [children: Vec<VNode<ViewData>>]);

fn my_component<ViewData: HasTuiViewData + Clone + 'static>((_c, MyComponentProps { title, children }): VComponentContext2<MyComponentProps<ViewData>, ViewData>) -> VNode<ViewData> {
    vbox!({ width: smt!(100%) }, {}, vec![
            text!({}, {}, title.clone()),
            vbox!({}, {}, children.clone())
        ])
}

#[test]
fn test_component() {
    let renderer = Renderer::new(TuiEngine::new(TuiConfig::default()));
    renderer.root(|(mut c, ())| my_component!(c, "key", { title: "Override title".to_owned() }, vec![
            text!({}, {}, "Hello world!".to_owned()),
        ]));
}
