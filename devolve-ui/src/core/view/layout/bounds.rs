use std::collections::HashMap;
use crate::core::view::layout::err::{LayoutError, LayoutResult};
use crate::core::view::layout::geom::{BoundingBox, Rectangle};
use crate::core::view::layout::parent_bounds::{DimsStore, LayoutDirection, ParentBounds};

#[derive(Clone, Eq, PartialEq)]
pub struct Bounds {
    layout: LayoutPosition,
    x: Measurement,
    y: Measurement,
    /// By default, the nodes are rendered next after prev, child after parent but before parent's sibling.
    /// Actually-specified z position takes precedence over this. If 2 nodes have the same z-position,
    /// they will be rendered as specified by the above.
    z: u32,
    anchor_x: f32,
    anchor_y: f32,
    width: Option<Measurement>,
    height: Option<Measurement>
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum LayoutPosition1D {
    GlobalAbsolute,
    LocalAbsolute,
    Relative
}

#[derive(Clone, Eq, PartialEq)]
pub struct LayoutPosition {
    x: LayoutPosition1D,
    y: LayoutPosition1D,
}

#[derive(Clone, Eq, PartialEq)]
pub enum Measurement {
    Zero,
    /// To the right of previous node if x, below if y, same as prev's if width or height
    Prev,
    Units(f32),
    Pixels(f32),
    /// Of parent size (must be known)
    Fraction(f32),
    Add(Box<Measurement>, Box<Measurement>),
    Sub(Box<Measurement>, Box<Measurement>),
    Mul(Box<Measurement>, f32),
    Div(Box<Measurement>, f32),
    /// Can be loaded by children via Load. Must be loaded from the same dimension (e.g. can't load height from width)
    Store(&'static str, Box<Measurement>),
    Load(&'static str)
}

const CHILD_Z: f32 = 0.0001f32;
const SIBLING_Z: f32 = 0.0000001f32;

#[derive(Clone, Copy)]
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
    pub fn resolve(&self, parent_bounds: &ParentBounds, prev_sibling: Option<&Rectangle>) -> LayoutResult<'_, (BoundingBox, DimsStore)> {
        let mut store = parent_bounds.store.clone();
        let bounding_box = BoundingBox {
            x: Self::apply_layout_x(parent_bounds, prev_sibling, self.layout.x, Self::reify_x(parent_bounds, &PrevSiblingDim::NotApplicable, Some(&mut store.x), &self.x).map_err(|err| err.add_store("x"))?).map_err(|err| err.add_store("x@layout"))?,
            y: Self::apply_layout_y(parent_bounds, prev_sibling, self.layout.y, Self::reify_y(parent_bounds, &PrevSiblingDim::NotApplicable, Some(&mut store.y), &self.y).map_err(|err| err.add_store("y"))?).map_err(|err| err.add_store("y@layout"))?,
            z: self.z + parent_bounds.bounding_box.z,
            anchor_x: self.anchor_x,
            anchor_y: self.anchor_y,
            width: self.width.map(|width| Self::reify_x(parent_bounds, prev_sibling.map(|r| r.width).into(), Some(&mut store.width), &width).map_err(|err| err.add_dimension("width"))).transpose()?,
            height: self.height.map(|height| Self::reify_y(parent_bounds, prev_sibling.map(|r| r.height).inti(), Some(&mut store.height), &height).map_err(|err| err.add_dimension("height"))).transpose()?
        };
        Ok((bounding_box, store))
    }

    fn reify_x(parent_bounds: &ParentBounds, prev_sibling: &PrevSiblingDim, mut store: Option<&mut HashMap<&'static str, f32>>, x: &Measurement) -> LayoutResult<'static, f32> {
        Ok(match x {
            Measurement::Zero => 0f32,
            Measurement::Prev => match prev_sibling {
                PrevSiblingDim::NotApplicable => Err(LayoutError::new("can't use prev for x: not applicable"))?,
                PrevSiblingDim::FirstChild => 0f32,
                PrevSiblingDim::Some(prev_sibling_dim) => prev_sibling_dim
            },
            Measurement::Units(x) => x,
            Measurement::Pixels(x) => x / parent_bounds.column_size.width,
            Measurement::Fraction(x) => match parent_bounds.bounding_box.width {
                None => Err(LayoutError::new("can't use fraction for x: parent width not known"))?,
                Some(width) => x * width,
            },
            Measurement::Add(lhs, rhs) => Self::reify_x(parent_bounds, prev_sibling, store.as_deref_mut(), lhs)? + Self::reify_x(parent_bounds, prev_sibling, store, rhs),
            Measurement::Sub(lhs, rhs) => Self::reify_x(parent_bounds, prev_sibling, store.as_deref_mut(), lhs)? - Self::reify_x(parent_bounds, prev_sibling, store, rhs),
            Measurement::Mul(lhs, scale) => Self::reify_x(parent_bounds, prev_sibling, store, lhs)? * scale,
            Measurement::Div(lhs, scale) => Self::reify_x(parent_bounds, prev_sibling, store, lhs)? / scale,
            Measurement::Store(name, x) => match store {
                None => Err(LayoutError::new("can't use store for x: dim-store not applicable"))?,
                Some(dim_store) => {
                    let result = Self::reify_x(parent_bounds, prev_sibling, Some(dim_store), x).map_err(|err| err.add_store(name))?;
                    dim_store.insert(name, result);
                    result
                }
            }
            Measurement::Load(name) => match store {
                None => Err(LayoutError::new("can't use load for x: dim-store not applicable")),
                Some(dim_store) => match dim_store.get(name) {
                    None => Err(LayoutError::new(format!("can't use load for x: no such dim {}", name)))?,
                    Some(result) => result
                }
            }
        })
    }

    fn reify_y(parent_bounds: &ParentBounds, prev_sibling: &PrevSiblingDim, mut store: Option<&mut HashMap<&'static str, f32>>, y: &Measurement) -> LayoutResult<'static, f32> {
        Ok(match y {
            Measurement::Zero => 0f32,
            Measurement::Prev => match prev_sibling {
                PrevSiblingDim::NotApplicable => Err(LayoutError::new("can't use prev for y: not applicable"))?,
                PrevSiblingDim::FirstChild => 0f32,
                PrevSiblingDim::Some(prev_sibling_dim) => prev_sibling_dim
            },
            Measurement::Units(y) => y,
            Measurement::Pixels(y) => y / parent_bounds.column_size.height,
            Measurement::Fraction(y) => match parent_bounds.bounding_box.height {
                None => Err(LayoutError::new("can't use fraction for y: parent height not known"))?,
                Some(height) => y * height
            },
            Measurement::Add(lhs, rhs) => Self::reify_y(parent_bounds, prev_sibling, store.as_deref_mut(), lhs)? + Self::reify_y(parent_bounds, prev_sibling, store, rhs),
            Measurement::Sub(lhs, rhs) => Self::reify_y(parent_bounds, prev_sibling, store.as_deref_mut(), lhs)? - Self::reify_y(parent_bounds, prev_sibling, store, rhs),
            Measurement::Mul(lhs, scale) => Self::reify_y(parent_bounds, prev_sibling, store, lhs)? * scale,
            Measurement::Div(lhs, scale) => Self::reify_y(parent_bounds, prev_sibling, store, lhs)? / scale,
            Measurement::Store(name, y) => match store {
                None => Err(LayoutError::new("can't use store for y: dim-store not applicable"))?,
                Some(store) => {
                    let result = Self::reify_y(parent_bounds, prev_sibling, Some(store), y).map_err(|err| err.add_store(name))?;
                    store.insert(name, result);
                    result
                }
            }
            Measurement::Load(name) => match store {
                None => Err(LayoutError::new("can't use load for y: dim-store not applicable"))?,
                Some(store) => match store.get(name) {
                    None => Err(LayoutError::new(format!("can't use load for y: no such dim {}", name)))?,
                    Some(result) => result
                }
            }
        })
    }

    fn apply_layout_x(parent_bounds: &ParentBounds, prev_sibling: Option<&Rectangle>, layout: LayoutPosition1D, reified: f32) -> LayoutResult<'static, f32> {
        Ok(match layout {
            LayoutPosition1D::GlobalAbsolute => reified,
            LayoutPosition1D::LocalAbsolute => reified + parent_bounds.bounding_box.x,
            LayoutPosition1D::Relative => match parent_bounds.sub_layout.direction {
                LayoutDirection::Horizontal => match prev_sibling {
                    None => reified  + parent_bounds.bounding_box.left().map_err(|err| err.add_dimension("parent.left"))?,
                    Some(prev_sibling) => {
                        // Yes, we do want to reify the parent's sub-layout with it's own bounds
                        let gap = Self::reify_x(parent_bounds, &PrevSiblingDim::NotApplicable, None, parent_bounds.sublayout.gap).map_err(|err| err.add_dimension("parent.gap"))?;
                        reified + prev_sibling.right + gap
                    }
                },
                LayoutDirection::Vertical => reified + parent_bounds.bounding_box.x,
                LayoutDirection::Overlap => reified + parent_bounds.bounding_box.x
            }
        })
    }

    fn apply_layout_y(parent_bounds: &ParentBounds, prev_sibling: Option<&Rectangle>, layout: LayoutPosition1D, reified: f32) -> LayoutResult<'static, f32> {
        Ok(match layout {
            LayoutPosition1D::GlobalAbsolute => reified,
            LayoutPosition1D::LocalAbsolute => reified + parent_bounds.bounding_box.y,
            LayoutPosition1D::Relative => match parent_bounds.sub_layout.direction {
                LayoutDirection::Horizontal => reified + parent_bounds.bounding_box.y,
                LayoutDirection::Vertical => match prev_sibling {
                    None => reified + parent_bounds.bounding_box.top().map_err(|err| err.add_dimension("parent.top"))?,
                    Some(prev_sibling) => {
                        // Yes, we do want to reify the parent's sub-layout with it's own bounds
                        let gap = Self::reify_y(parent_bounds, &PrevSiblingDim::NotApplicable, None, parent_bounds.sublayout.gap).map_err(|err| err.add_dimension("parent.gap"))?;
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
            layout: LayoutPosition::default(),
            x: Measurement::Zero,
            y: Measurement::Zero,
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

impl Default for LayoutPosition {
    fn default() -> Self {
        LayoutPosition {
            x: LayoutPosition1D::default(),
            y: LayoutPosition1D::default()
        }
    }
}
