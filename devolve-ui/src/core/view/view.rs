use std::borrow::Cow;
use std::fmt;
use std::fmt::{Display, Formatter};
use crate::core::component::node::{NodeId, VNode};
use crate::core::view::layout::bounds::Bounds;
use crate::core::view::layout::parent_bounds::SubLayout;

pub struct VView<ViewData: VViewData> {
    pub id: NodeId,
    pub bounds: Bounds,
    pub is_visible: bool,
    pub key: Option<Cow<'static, str>>,
    pub d: ViewData
}

#[derive(Debug, Clone, Copy, P)]
pub struct VViewType(&'static str);

impl Display for VViewType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub trait VViewData {
    type Children: Iterator<Item=&VNode<Self>>;
    type ChildrenMut: Iterator<Item=&mut VNode<Self>>;

    fn typ(&self) -> VViewType;
    fn children(&self) -> Option<(Children, SubLayout)>;
    fn children_mut(&mut self) -> Option<(ChildrenMut, SubLayout)>;
}

/*pub enum VViewType {
    Box {
        children: Vec<VNode>,
        // sub_layout: SubLayout,
        // clip: bool,
        // extend: bool
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
    },
    Custom(dyn VViewCustom)
}*/