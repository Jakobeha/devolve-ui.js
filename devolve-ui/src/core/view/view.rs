use crate::core::component::node::VNode;
// use crate::core::view::border_style::{BorderStyle, DividerStyle};

pub struct VView {
    id: usize,
    // bounds: Bounds,
    // visible: bool,
    // key: Option<Cow<'_, str>>,
    pub t: dyn VViewType
}

pub trait VViewType {
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