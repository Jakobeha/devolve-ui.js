use std::collections::HashMap;
use crate::core::view::layout::err::{LayoutError, LayoutResult};
use crate::core::view::layout::geom::{BoundingBox, Rectangle};
use crate::core::view::layout::measurement::{Measurement, MeasurementUnit};
use crate::core::view::layout::parent_bounds::{DimsStore, LayoutDirection, ParentBounds};

#[derive(Debug, Clone, PartialEq)]
pub struct Bounds {
    pub layout: LayoutPosition,
    pub x: Measurement,
    pub y: Measurement,
    /// By default, the nodes are rendered next after prev, child after parent but before parent's sibling.
    /// Actually-specified z position takes precedence over this. If 2 nodes have the same z-position,
    /// they will be rendered as specified by the above.
    pub z: i32,
    pub anchor_x: f32,
    pub anchor_y: f32,
    pub width: Option<Measurement>,
    pub height: Option<Measurement>
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum LayoutPosition1D {
    Relative,
    GlobalAbsolute,
    LocalAbsolute
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct LayoutPosition {
    pub x: LayoutPosition1D,
    pub y: LayoutPosition1D,
}

const MAX_CHILDREN_EXPECTED_LOG2: f64 = 8f64;

#[derive(Debug, Clone, Copy)]
enum PrevSiblingDim {
    NotApplicable,
    FirstChild,
    Some(f32)
}

impl From<Option<f32>> for PrevSiblingDim {
    fn from(prev_sibling_dim: Option<f32>) -> Self {
        match prev_sibling_dim {
            None => PrevSiblingDim::FirstChild,
            Some(prev_sibling_dim) => PrevSiblingDim::Some(prev_sibling_dim),
        }
    }
}

impl Bounds {
    pub fn resolve(&self, parent_bounds: &ParentBounds, prev_sibling: Option<&Rectangle>, parent_depth: usize, sibling_index: usize) -> LayoutResult<(BoundingBox, DimsStore)> {
        let mut store = parent_bounds.store.clone();
        let bounding_box = BoundingBox {
            x: Self::apply_layout_x(parent_bounds, prev_sibling, self.layout.x, Self::reify_x(parent_bounds, &PrevSiblingDim::NotApplicable, Some(&mut store.x), &self.x).map_err(|err| err.add_store("x"))?).map_err(|err| err.add_store("x@layout"))?,
            y: Self::apply_layout_y(parent_bounds, prev_sibling, self.layout.y, Self::reify_y(parent_bounds, &PrevSiblingDim::NotApplicable, Some(&mut store.y), &self.y).map_err(|err| err.add_store("y"))?).map_err(|err| err.add_store("y@layout"))?,
            z: self.z as f64 + parent_bounds.bounding_box.z - f64::floor(parent_bounds.bounding_box.z) + (((parent_depth + 1) as f64 * -MAX_CHILDREN_EXPECTED_LOG2).exp2() * (sibling_index + 1) as f64),
            anchor_x: self.anchor_x,
            anchor_y: self.anchor_y,
            width: self.width.as_ref().map(|width| Self::reify_x(parent_bounds, &prev_sibling.map(|r| r.width()).into(), Some(&mut store.width), &width).map_err(|err| err.add_dimension("width"))).transpose()?.into(),
            height: self.height.as_ref().map(|height| Self::reify_y(parent_bounds, &prev_sibling.map(|r| r.height()).into(), Some(&mut store.height), &height).map_err(|err| err.add_dimension("height"))).transpose()?.into()
        };
        Ok((bounding_box, store))
    }

    fn reify_x(parent_bounds: &ParentBounds, prev_sibling: &PrevSiblingDim, store: Option<&mut HashMap<&'static str, f32>>, x: &Measurement) -> LayoutResult<f32> {
        let mut reified = 0f32;
        for x in x.iter_adds() {
            reified += x.value.scalar() * match x.unit {
                MeasurementUnit::Units => 1f32,
                MeasurementUnit::Pixels => 1f32 / parent_bounds.column_size.width,
                MeasurementUnit::PercentOfParent => match parent_bounds.bounding_box.width.into_option() {
                    None => Err(LayoutError::new("can't use fraction for x: parent width not known"))?,
                    Some(width) => width / 100f32
                },
                MeasurementUnit::OfPrev => match prev_sibling {
                    PrevSiblingDim::NotApplicable => Err(LayoutError::new("can't use prev for x: not applicable"))?,
                    PrevSiblingDim::FirstChild => 0f32,
                    PrevSiblingDim::Some(prev_sibling_dim) => *prev_sibling_dim
                },
                MeasurementUnit::OfLoad(ident) => match &store {
                    None => Err(LayoutError::new("can't load y: dim-store not applicable"))?,
                    Some(store) => match store.get(ident) {
                        None => Err(LayoutError::new(format!("can't load y: no such dim {}", ident)))?,
                        Some(result) => *result
                    }
                }
            };
        }
        if let Some(ident) = x.store {
            match store {
                None => Err(LayoutError::new("can't store x: dim-store not applicable"))?,
                Some(store) => store.insert(ident, reified)
            };
        }
        Ok(reified)
    }

    fn reify_y(parent_bounds: &ParentBounds, prev_sibling: &PrevSiblingDim, store: Option<&mut HashMap<&'static str, f32>>, y: &Measurement) -> LayoutResult<f32> {
        let mut reified = 0f32;
        for y in y.iter_adds() {
            reified += y.value.scalar() * match y.unit {
                MeasurementUnit::Units => 1f32,
                MeasurementUnit::Pixels => 1f32 / parent_bounds.column_size.height,
                MeasurementUnit::PercentOfParent => match parent_bounds.bounding_box.height.into_option() {
                    None => Err(LayoutError::new("can't use fraction for y: parent height not known"))?,
                    Some(height) => height / 100f32
                },
                MeasurementUnit::OfPrev => match prev_sibling {
                    PrevSiblingDim::NotApplicable => Err(LayoutError::new("can't use prev for y: not applicable"))?,
                    PrevSiblingDim::FirstChild => 0f32,
                    PrevSiblingDim::Some(prev_sibling_dim) => *prev_sibling_dim
                }
                MeasurementUnit::OfLoad(ident) => match &store {
                    None => Err(LayoutError::new("can't load y: dim-store not applicable"))?,
                    Some(store) => match store.get(ident) {
                        None => Err(LayoutError::new(format!("can't load y: no such dim {}", ident)))?,
                        Some(result) => *result
                    }
                }
            }
        }
        if let Some(ident) = y.store {
            match store {
                None => Err(LayoutError::new("can't store y: dim-store not applicable"))?,
                Some(store) => store.insert(ident, reified)
            };
        }
        Ok(reified)
    }

    fn apply_layout_x(parent_bounds: &ParentBounds, prev_sibling: Option<&Rectangle>, layout: LayoutPosition1D, reified: f32) -> LayoutResult<f32> {
        Ok(match layout {
            LayoutPosition1D::GlobalAbsolute => reified,
            LayoutPosition1D::LocalAbsolute => reified + parent_bounds.bounding_box.x,
            LayoutPosition1D::Relative => match parent_bounds.sub_layout.direction {
                LayoutDirection::Horizontal => match prev_sibling {
                    None => reified + parent_bounds.bounding_box.left().map_err(|err| err.add_dimension("parent.left"))?,
                    Some(prev_sibling) => {
                        // Yes, we do want to reify the parent's sub-layout with it's own bounds
                        let gap = Self::reify_x(parent_bounds, &PrevSiblingDim::NotApplicable, None, &parent_bounds.sub_layout.gap).map_err(|err| err.add_dimension("parent.gap"))?;
                        reified + prev_sibling.right + gap
                    }
                },
                LayoutDirection::Vertical => reified + parent_bounds.bounding_box.x,
                LayoutDirection::Overlap => reified + parent_bounds.bounding_box.x
            }
        })
    }

    fn apply_layout_y(parent_bounds: &ParentBounds, prev_sibling: Option<&Rectangle>, layout: LayoutPosition1D, reified: f32) -> LayoutResult<f32> {
        Ok(match layout {
            LayoutPosition1D::GlobalAbsolute => reified,
            LayoutPosition1D::LocalAbsolute => reified + parent_bounds.bounding_box.y,
            LayoutPosition1D::Relative => match parent_bounds.sub_layout.direction {
                LayoutDirection::Horizontal => reified + parent_bounds.bounding_box.y,
                LayoutDirection::Vertical => match prev_sibling {
                    None => reified + parent_bounds.bounding_box.top().map_err(|err| err.add_dimension("parent.top"))?,
                    Some(prev_sibling) => {
                        // Yes, we do want to reify the parent's sub-layout with it's own bounds
                        let gap = Self::reify_y(parent_bounds, &PrevSiblingDim::NotApplicable, None, &parent_bounds.sub_layout.gap).map_err(|err| err.add_dimension("parent.gap"))?;
                        reified + prev_sibling.bottom + gap
                    }
                },
                LayoutDirection::Overlap => reified + parent_bounds.bounding_box.y
            }
        })
    }
}

impl LayoutPosition {
    pub fn xy(xy: LayoutPosition1D) -> LayoutPosition {
        return LayoutPosition {
            x: xy,
            y: xy
        }
    }
}

impl Default for Bounds {
    fn default() -> Self {
        Bounds {
            layout: LayoutPosition::xy(LayoutPosition1D::Relative),
            x: Measurement::ZERO,
            y: Measurement::ZERO,
            z: 0,
            anchor_x: 0f32,
            anchor_y: 0f32,
            width: None,
            height: None
        }
    }
}


impl Default for LayoutPosition1D {
    fn default() -> Self {
        LayoutPosition1D::Relative
    }
}