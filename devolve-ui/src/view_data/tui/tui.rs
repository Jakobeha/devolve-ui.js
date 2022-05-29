use crate::core::component::node::VNode;
use crate::core::view::color::Color;
use crate::core::view::layout::parent_bounds::SubLayout;
use crate::core::view::view::{VViewData, VViewType};
use crate::view_data::attrs::{BorderStyle, DividerStyle, TextWrapMode};
use std::slice::{Iter, IterMut};

pub enum TuiViewData {
    Box {
        children: Vec<VNode<TuiViewData>>,
        sub_layout: SubLayout,
        clip: bool,
        extend: bool,
    },
    Text {
        text: String,
        color: Option<Color>,
        wrap_mode: TextWrapMode,
    },
    Color {
        color: Color,
    },
    Border {
        color: Option<Color>,
        style: BorderStyle,
    },
    Divider {
        color: Option<Color>,
        style: DividerStyle,
    },
    Source {
        source: String,
    },
}

impl VViewData for TuiViewData {
    type Children<'a> = Iter<'a, VNode<Self>>;
    type ChildrenMut<'a> = IterMut<'a, VNode<Self>>;

    fn typ(&self) -> VViewType {
        match self {
            TuiViewData::Box { .. } => VViewType::from("Tui::Box"),
            TuiViewData::Text { .. } => VViewType::from("Tui::Text"),
            TuiViewData::Color { .. } => VViewType::from("Tui::Color"),
            TuiViewData::Border { .. } => VViewType::from("Tui::Border"),
            TuiViewData::Divider { .. } => VViewType::from("Tui::Divider"),
            TuiViewData::Source { .. } => VViewType::from("Tui::Source"),
        }
    }

    fn children(&self) -> Option<(Self::Children<'_>, SubLayout)> {
        match self {
            TuiViewData::Box {
                children,
                sub_layout,
                ..
            } => Some((children.iter(), sub_layout.clone())),
            _ => None,
        }
    }

    fn children_mut(&mut self) -> Option<(Self::ChildrenMut<'_>, SubLayout)> {
        match self {
            TuiViewData::Box {
                children,
                sub_layout,
                ..
            } => Some((children.iter_mut(), sub_layout.clone())),
            _ => None,
        }
    }
}
