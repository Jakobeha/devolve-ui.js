use std::borrow::Cow;
use devolve_ui::core::component::component::VComponent;
use devolve_ui::core::component::node::VNode;
use devolve_ui::core::renderer::renderer::Renderer;
use devolve_ui::core::view::layout::bounds::{Bounds, Measurement};
use devolve_ui::core::view::layout::parent_bounds::{LayoutDirection, SubLayout};
use devolve_ui::core::view::view::VView;
use devolve_ui::engines::tui::tui::{TuiConfig, TuiEngine};
use devolve_ui::view_data::tui::TuiViewData;

struct WordleProps {
    text: String,
}

fn wordle_component(c: &mut Box<VComponent<TuiViewData>>) -> VNode<TuiViewData> {
    VNode::View(Box::new(VView::new(
        Bounds::default(),
        true,
        None,
        TuiViewData::Box {
            children: vec![],
            sub_layout: SubLayout {
                direction: LayoutDirection::Horizontal,
                gap: Measurement::Zero
            },
            clip: false,
            extend: false
        }
    )))
}

#[test]
fn wordle() {
    let mut renderer = Renderer::new(TuiEngine::new(TuiConfig::default()));
    renderer.root(|parent| VComponent::new(parent, &Cow::Borrowed("wordle_root"), WordleProps { text: "Hello world".into() }, wordle_component));
    // renderer.interval_between_frames(Duration::from_millis(25)); // optional
    renderer.show();
    // renderer.resume();

}