use std::borrow::Cow;
use std::fmt;
use std::fmt::{Display, Formatter};
use crate::core::component::node::{NodeId, VNode};
use crate::core::view::layout::bounds::Bounds;
use crate::core::view::layout::parent_bounds::SubLayout;

#[derive(Debug)]
pub struct VView<ViewData: VViewData> {
    id: NodeId,
    pub bounds: Bounds,
    pub is_visible: bool,
    pub key: Option<Cow<'static, str>>,
    pub d: ViewData
}

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct VViewType(&'static str);

impl VViewType {
    pub fn from(str: &'static str) -> VViewType {
        VViewType(str)
    }
}

impl Display for VViewType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub trait VViewData: Sized {
    type Children<'a>: Iterator<Item=&'a VNode<Self>> where Self: 'a;
    type ChildrenMut<'a>: Iterator<Item=&'a mut VNode<Self>> where Self: 'a;

    fn typ(&self) -> VViewType;
    fn children(&self) -> Option<(Self::Children<'_>, SubLayout)>;
    fn children_mut(&mut self) -> Option<(Self::ChildrenMut<'_>, SubLayout)>;
}

impl <ViewData: VViewData> VView<ViewData> {
    pub fn new(bounds: Bounds, is_visible: bool, key: Option<Cow<'static, str>>, d: ViewData) -> VView<ViewData> {
        VView {
            id: VNode::<ViewData>::next_id(),
            bounds,
            is_visible,
            key,
            d
        }
    }

    pub fn id(&self) -> NodeId {
        self.id
    }
}