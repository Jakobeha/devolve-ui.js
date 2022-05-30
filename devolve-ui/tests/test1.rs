#![feature(decl_macro)]
#![feature(macro_metavar_expr)]
#![cfg(feature = "tui")]

use std::borrow::Cow;
use devolve_ui::core::component::component::VComponent;
use devolve_ui::core::component::constr::make_component;
use devolve_ui::core::component::node::VNode;
use devolve_ui::core::renderer::renderer::Renderer;
use devolve_ui::core::view::layout::bounds::{Bounds, Measurement};
use devolve_ui::core::view::layout::parent_bounds::{LayoutDirection, SubLayout};
use devolve_ui::core::view::view::VView;
use devolve_ui::engines::tui::tui::{TuiConfig, TuiEngine};
use devolve_ui::view_data::tui::tui::TuiViewData;
use devolve_ui::view_data::tui::constr::{vbox, text};

struct WordleProps {
    text: String,
}

make_component!(pub wordle, WordleProps { text: String }, { text: Default::default() }, <TuiViewData>|c, text| {
    vbox!({}, {}, [
        text!("Hello world!")
    ])
});

#[test]
fn wordle() {
    let mut renderer = Renderer::new(TuiEngine::new(TuiConfig::default()));
    renderer.root(|c| wordle!(wordle_fn, { text: "Hello world".into() }));
    // renderer.interval_between_frames(Duration::from_millis(25)); // optional
    renderer.show();
    // renderer.resume();

}