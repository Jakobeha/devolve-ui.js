use std::borrow::Cow;
use std::fmt;
use std::fmt::{Display, Formatter};
use crate::core::component::node::{NodeId, VNode};
use crate::core::view::layout::bounds::Bounds;
// use crate::core::view::border_style::{BorderStyle, DividerStyle};

pub struct VView<ViewData: VViewData> {
    id: NodeId,
    bounds: Bounds,
    is_visible: bool,
    key: Option<Cow<'static, str>>,
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
    fn children(&self) -> Children;
    fn children_mut(&mut self) -> ChildrenMut;
}

/*pub enum VViewType {
    Box {
        children: Vec<VNode>,
        // sublayout: SubLayout,
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

impl <ViewData: VViewData> VView<ViewData> {
    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn children(&self) -> ViewData::Children {
        self.t.children()
    }

    pub fn children_mut(&mut self) -> ViewData::ChildrenMut {
        self.t.children_mut()
    }
}