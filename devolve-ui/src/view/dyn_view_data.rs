//! Helpers if you don't know the type of `ViewData` you want to provide for components.
//! Since `ViewData` is unsized we can provide raw bytes which can be interpreted as any type with a runtime vtable.

use crate::component::node::VNode;
use crate::view::layout::parent_bounds::SubLayout;
use crate::view::view::{VViewData, VViewType};

pub struct DynViewDataVTable {
    typ: fn(&[u8]) -> VViewType,
    children: fn(&[u8]) -> Option<(&[VNode<DynViewData>], SubLayout)>,
    children_mut: fn(&[u8]) -> Option<(&mut [VNode<DynViewData>], SubLayout)>,
}

pub struct DynViewData {
    vtable: &'static DynViewDataVTable,
    data: [u8]
}

impl VViewData for DynViewData {
    type Children<'a> = std::slice::Iter<'a, VNode<DynViewData>>;
    type ChildrenMut<'a> = std::slice::IterMut<'a, VNode<DynViewData>>;

    fn typ(&self) -> VViewType {
        (self.vtable.typ)(&self.data)
    }

    fn children(&self) -> Option<(Self::Children<'_>, SubLayout)> {
        let children_slice = (self.vtable.children)(&self.data);
        children_slice.map(|(children, layout)| (children.iter(), layout))
    }

    fn children_mut(&mut self) -> Option<(Self::ChildrenMut<'_>, SubLayout)> {
        let children_slice = (self.vtable.children_mut)(&self.data);
        children_slice.map(|(children, layout)| (children.iter_mut(), layout))
    }
}
