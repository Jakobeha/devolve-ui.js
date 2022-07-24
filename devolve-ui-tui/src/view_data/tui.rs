use crate::component::node::VNode;
use crate::view::color::Color;
use crate::view::layout::parent_bounds::SubLayout;
use crate::view::view::{VViewData, VViewType};
use crate::view_data::attrs::{BorderStyle, DividerDirection, DividerStyle, TextWrapMode};
use std::slice::{Iter, IterMut};
#[cfg(feature = "tui-images")]
use crate::view_data::tui::terminal_image::{HandleAspectRatio, Source};

#[derive(Debug, Clone)]
pub struct TuiBoxAttrs {
    pub sub_layout: SubLayout,
    pub clip: bool,
    pub extend: bool,
}

#[derive(Debug, Clone)]
pub enum TuiViewData {
    Box {
        children: Vec<VNode<TuiViewData>>,
        attrs: TuiBoxAttrs
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
        direction: DividerDirection,
        style: DividerStyle,
    },
    #[cfg(feature = "tui-images")]
    Source {
        source: Source,
        handle_aspect_ratio: HandleAspectRatio
    },
}

pub trait HasTuiBox: VViewData {
    fn tui_box(children: Vec<VNode<Self>>, attrs: TuiBoxAttrs) -> Self;
    #[allow(clippy::needless_lifetimes)]
    fn as_tui_box<'a>(&'a self) -> Option<(&'a Vec<VNode<Self>>, &'a TuiBoxAttrs)>;
}

impl HasTuiBox for TuiViewData {
    fn tui_box(children: Vec<VNode<Self>>, attrs: TuiBoxAttrs) -> Self {
        TuiViewData::Box { children, attrs }
    }

    #[allow(clippy::needless_lifetimes)]
    fn as_tui_box<'a>(&'a self) -> Option<(&'a Vec<VNode<Self>>, &'a TuiBoxAttrs)> {
        match self {
            TuiViewData::Box { children, attrs } => Some((children, attrs)),
            _ => None
        }
    }
}

pub trait HasTuiViewData: VViewData + From<TuiViewData> + HasTuiBox {}

impl HasTuiViewData for TuiViewData {}

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
            #[cfg(feature = "tui-images")]
            TuiViewData::Source { .. } => VViewType::from("Tui::Source"),
        }
    }

    fn children(&self) -> Option<(Self::Children<'_>, SubLayout)> {
        match self {
            TuiViewData::Box {
                children,
                attrs,
                ..
            } => Some((children.iter(), attrs.sub_layout.clone())),
            _ => None,
        }
    }

    fn children_mut(&mut self) -> Option<(Self::ChildrenMut<'_>, SubLayout)> {
        match self {
            TuiViewData::Box {
                children,
                attrs,
                ..
            } => Some((children.iter_mut(), attrs.sub_layout.clone())),
            _ => None,
        }
    }
}
