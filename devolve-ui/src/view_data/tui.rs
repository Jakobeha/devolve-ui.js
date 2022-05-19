use std::iter::empty;
use crate::core::component::node::VNode;
use crate::view_data::border_style::{BorderStyle, DividerStyle};
use crate::core::view::color::Color;
use crate::core::view::layout::parent_bounds::SubLayout;
use crate::core::view::view::{VViewData, VViewType};

pub enum TuiViewData<Self_: VViewData> {
    Box {
        children: Vec<VNode<Self_>>,
        sublayout: SubLayout,
        clip: bool,
        extend: bool
    },
    Text {
        text: String,
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

impl <Self_: VViewData> VViewData for TuiViewData<Self_> {
    type Children = Box<dyn Iterator<Item=&VNode<Self>>>;
    type ChildrenMut = Box<dyn Iterator<Item=&mut VNode<Self>>>;


    fn typ(&self) -> VViewType {
        match self {
            TuiViewData::Box { .. } => VViewType("Tui::Box"),
            TuiViewData::Text { .. } => VViewType("Tui::Text"),
            TuiViewData::Color { .. } => VViewType("Tui::Color"),
            TuiViewData::Border { .. } => VViewType("Tui::Border"),
            TuiViewData::Divider { .. } => VViewType("Tui::Divider"),
            TuiViewData::Source { .. } => VViewType("Tui::Source"),
        }
    }

    fn children(&self) -> Self::Children {
        Box::new(match self {
            TuiViewData::Box { children, .. } => children.iter(),
            _ => empty()
        })
    }

    fn children_mut(&mut self) -> Self::ChildrenMut {
        Box::new(match self {
            TuiViewData::Box { children, .. } => children.iter_mut(),
            _ => empty()
        })
    }
}