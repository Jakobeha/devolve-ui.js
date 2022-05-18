use std::fmt;
use std::fmt::{Display, Formatter};
use crate::core::component::node::VNode;
// use crate::core::view::border_style::{BorderStyle, DividerStyle};

pub struct VView {
    id: usize,
    // bounds: Bounds,
    // visible: bool,
    // key: Option<Cow<'_, str>>,
    pub d: dyn VViewData
}

#[derive(Debug, Clone, Copy, P)]
pub struct VViewType(&'static str);

impl Display for VViewType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub trait VViewData {
    fn typ(&self) -> VViewType;
    fn children(&self) -> &Vec<VNode>;
    fn children_mut(&mut self) -> &mut Vec<VNode>;
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

impl VView {
    pub fn id(&self) -> usize {
        self.id
    }

    pub fn children(&self) -> &Vec<VNode> {
        self.t.children()
    }

    pub fn children_mut(&mut self) -> &mut Vec<VNode> {
        self.t.children_mut()
    }
}