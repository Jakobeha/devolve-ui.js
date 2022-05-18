use std::collections::HashMap;
use crate::core::view::layout::bounds::Measurement;
use crate::core::view::layout::geom::{BoundingBox, Size};

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum LayoutDirection {
    Horizontal,
    Vertical,
    Overlap
}

#[derive(Clone, Eq, PartialEq)]
pub struct SubLayout {
    pub direction: LayoutDirection,
    pub gap: Option<Measurement>
}

#[derive(Clone, Eq, PartialEq)]
pub struct DimMap<T> {
    pub x: T,
    pub y: T,
    pub width: T,
    pub height: T,
}

pub type DimsStore = DimMap<HashMap<&'static str, f32>>;

#[derive(Clone, Eq, PartialEq)]
pub struct ParentBounds {
    pub bounding_box: BoundingBox,
    pub sub_layout: SubLayout,
    pub column_size: Size,
    pub store: DimsStore
}