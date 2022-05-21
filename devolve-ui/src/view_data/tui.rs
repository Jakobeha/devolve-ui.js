use std::slice::{Iter, IterMut};
use crate::core::component::node::VNode;
use crate::view_data::attrs::{BorderStyle, DividerStyle, TextWrapMode};
use crate::core::view::color::Color;
use crate::core::view::layout::parent_bounds::SubLayout;
use crate::core::view::view::{VViewData, VViewType};

pub enum TuiViewData<'a, Self_: VViewData<'a>> {
    Box {
        children: Vec<VNode<Self_>>,
        sub_layout: SubLayout,
        clip: bool,
        extend: bool
    },
    Text {
        text: String,
        color: Color,
        wrap_mode: TextWrapMode
    },
    Color {
        color: Color
    },
    Border {
        color: Color,
        style: BorderStyle
    },
    Divider {
        color: Color,
        style: DividerStyle
    },
    Source {
        source: String
    }
}

impl <'a, Self_: VViewData<'a>> VViewData<'a> for TuiViewData<Self_> {
    type Children = Iter<'a, VNode<Self_>>;
    type ChildrenMut = IterMut<'a, VNode<Self_>>;


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

    fn children(&self) -> Option<(Self::Children, SubLayout)> {
        match self {
            TuiViewData::Box { children, sub_layout, .. } => Some((children.iter(), sub_layout.clone())),
            _ => None
        }
    }

    fn children_mut(&mut self) -> Option<(Self::ChildrenMut, SubLayout)> {
        match self {
            TuiViewData::Box { children, sub_layout, .. } => Some((children.iter_mut(), sub_layout.clone())),
            _ => None
        }
    }
}