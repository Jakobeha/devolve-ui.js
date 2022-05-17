use crate::core::component::node::VNode;
use crate::core::view::border_style::BorderStyle;
use crate::core::view::color::Color;

pub struct VView {
    id: usize,
    // bounds: Bounds,
    // visible: bool,
    // key: Option<Cow<'_, str>>,
    pub t: VViewType
}

pub enum VViewType {
    VBox {
        children: Vec<VNode>,
        // sublayout: SubLayout,
        // clip: bool,
        // extend: bool
    },
    VText {
        text: String,
    },
    VColor {
        color: Color
    },
    Border {
        color: Color,
        style: BorderStyle
    },
    Source {
        source: String
    }
}

impl VView {
    pub fn id(&self) -> usize {
        self.id
    }
}