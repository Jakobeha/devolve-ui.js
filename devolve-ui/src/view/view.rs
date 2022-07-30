//! Views are graphical elements drawn to the screen, including text, images, and more.
//! Views are also used to layout other views: views can also contain multiple children,
//! and influence how and where (and even when) the children are drawn.
//!
//! The specific type of views available depends on `ViewData`.

use std::fmt;
use std::fmt::{Display, Formatter};
use crate::component::node::{NodeId, VNode};
use crate::view::layout::bounds::Bounds;
use crate::view::layout::parent_bounds::SubLayout;
#[cfg(feature = "logging")]
use serde::{Serialize, Deserialize};
use dyn_struct2::dyn_arg::DynArg;
use dyn_struct2::DynStruct;

#[repr(C)]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VView<ViewData: VViewData + ?Sized> {
    id: NodeId,
    pub bounds: Bounds,
    pub is_visible: bool,
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

pub trait VViewData {
    type Children<'a>: Iterator<Item=&'a VNode<Self>> where Self: 'a;
    type ChildrenMut<'a>: Iterator<Item=&'a mut VNode<Self>> where Self: 'a;

    fn typ(&self) -> VViewType;
    fn children(&self) -> Option<(Self::Children<'_>, SubLayout)>;
    fn children_mut(&mut self) -> Option<(Self::ChildrenMut<'_>, SubLayout)>;
}

impl <ViewData: VViewData + ?Sized> VView<ViewData> {
    pub fn new(bounds: Bounds, is_visible: bool, d: ViewData) -> VView<ViewData> where ViewData: Sized {
        VView {
            id: NodeId::next(),
            bounds,
            is_visible,
            d
        }
    }

    pub fn new_unsized(bounds: Bounds, is_visible: bool, d: DynArg<ViewData>) -> Box<VView<ViewData>> {
        let dyn_struct = DynStruct::<(NodeId, Bounds, bool), ViewData>::new(
            (NodeId::next(), bounds, is_visible), d
        );

        unsafe { dyn_struct.more_unsafe_transmute() }
    }

    pub fn id(&self) -> NodeId {
        self.id
    }
}