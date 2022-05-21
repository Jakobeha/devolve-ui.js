use std::borrow::Cow;
use std::collections::HashMap;
use crate::core::view::layout::bounds::Measurement;
use crate::core::view::layout::geom::{BoundingBox, Size};

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum LayoutDirection {
    Horizontal,
    Vertical,
    Overlap
}

#[derive(Clone, PartialEq)]
pub struct SubLayout {
    pub direction: LayoutDirection,
    pub gap: Measurement
}

#[derive(Clone, PartialEq)]
pub struct DimMap<T> {
    pub x: T,
    pub y: T,
    pub width: T,
    pub height: T,
}

impl <T: Default> DimMap<T> {
    pub fn new() -> Self {
        DimMap {
            x: Default::default(),
            y: Default::default(),
            width: Default::default(),
            height: Default::default(),
        }
    }
}

pub type DimsStore = DimMap<HashMap<&'static str, f32>>;

#[derive(Clone, PartialEq)]
pub struct ParentBounds {
    pub bounding_box: BoundingBox,
    pub sub_layout: SubLayout,
    pub column_size: Cow<'static, Size>,
    pub store: DimsStore
}

impl ParentBounds {
    pub fn typical_root(size: Size, column_size: Size, store: DimsStore) -> ParentBounds {
        ParentBounds {
            bounding_box: BoundingBox {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                anchor_x: 0.0,
                anchor_y: 0.0,
                width: Some(size.width),
                height: Some(size.height)
            },
            sub_layout: SubLayout {
                direction: LayoutDirection::Vertical,
                gap: Measurement::Zero
            },
            column_size: Cow::Owned(column_size),
            store
        }
    }
}