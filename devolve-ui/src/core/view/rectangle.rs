use std::borrow::Cow;
use std::collections::HashMap;
use crate::core::component::component::VComponentKey;
use crate::core::view::view::VViewType;

#[derive(Debug, Clone)]
pub struct LayoutError<'a> {
    message: Cow<'a, str>,
    path: String,
}

pub type LayoutResult<'a, T> = Result<T, LayoutError<'a>>;

impl LayoutError {
    pub fn new<'a, Str: Into<Cow<'a, str>>>(message: Str) -> LayoutError<'a> {
        LayoutError {
            message: message.into(),
            path: String::new(),
        }
    }

    pub fn add_dimension(&self, dimension: &str) -> LayoutError {
        LayoutError {
            message: self.message.clone(),
            path: format!("{}{}", dimension, self.path),
        }
    }

    pub fn add_store(&self, store: &str) -> LayoutError {
        LayoutError {
            message: self.message.clone(),
            path: format!("{}${}", self.path, store),
        }
    }

    pub fn add_component(&self, parent_key: &VComponentKey, parent_id: usize) -> LayoutError {
        LayoutError {
            message: self.message.clone(),
            path: format!("{}#{}.{}", parent_key.to_string(), parent_id, self.path),
        }
    }

    pub fn add_view(&self, parent_type: VViewType, parent_id: usize, index: usize) -> LayoutError {
        LayoutError {
            message: self.message.clone(),
            path: format!("{}#{}[{}].{}", parent_type.to_string(), parent_id, index, self.path),
        }
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub anchor_x: f32,
    pub anchor_y: f32,
}

#[derive(Clone, Eq, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Eq, PartialEq)]
pub struct Rectangle {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

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
            x: Self::apply_layout_x(parent_bounds, prev_sibling, self.layout.x, Self::reify_x(parent_bounds, PrevSibling::NotApplicable, Some(&mut store.x), &self.x).map_err(|err| err.add_store("x"))?).map_err(|err| err.add_store("x@layout"))?,
            y: Self::apply_layout_y(parent_bounds, prev_sibling, self.layout.y, Self::reify_y(parent_bounds, PrevSibling::NotApplicable, Some(&mut store.y), &self.y).map_err(|err| err.add_store("y"))?).map_err(|err| err.add_store("y@layout"))?,
            z: self.z + parent_bounds.bounding_box.z,
            anchor_x: self.anchor_x,
            anchor_y: self.anchor_y,
            width: self.width.map(|width| Self::reify_x(parent_bounds, prev_sibling.map(|r| r.width).into(), Some(&mut store.width), &width).map_err(|err| err.add_dimension("width"))).transpose()?,
            height: self.height.map(|height| Self::reify_y(parent_bounds, prev_sibling.map(|r| r.height).inti(), Some(&mut store.height), &height).map_err(|err| err.add_dimension("height"))).transpose()?
        };
        Ok((bounding_box, store))
    }

    fn reify_x(parent_bounds: &ParentBounds, prev_sibling: &PrevSiblingDim, dim_store: Option<&mut HashMap<&'static str, f32>>, x: &Measurement) -> LayoutResult<'_, f32> {
        Ok(match x {
            Measurement::Zero => 0f32,
            Measurement::Prev => match prev_sibling {
                PrevSiblingDim::NotApplicable => Err(LayoutError::new("can't use prev for x: not applicable")),
                PrevSiblingDim::FirstChild => 0f32,
                PrevSiblingDim::Some(prev_sibling_dim) => prev_sibling_dim
            },
            Measurement::Units(x) => x,
            Measurement::Pixels(x) => x / parent_bounds.column_size.width,
            Measurement::Fraction(x) => match parent_bounds.bounding_box.width {
                None => Err(LayoutError::new("can't use fraction for x: parent width not known")),
                Some(width) => x * width,
            },
            Measurement::Add(lhs, rhs) => Self::reify_x(parent_bounds, prev_sibling, store, lhs)? + Self::reify_x(parent_bounds, prev_sibling, store, rhs),
            Measurement::Sub(lhs, rhs) => Self::reify_x(parent_bounds, prev_sibling, store, lhs)? - Self::reify_x(parent_bounds, prev_sibling, store, rhs),
            Measurement::Mul(lhs, scale) => Self::reify_x(parent_bounds, prev_sibling, store, lhs)? * scale,
            Measurement::Div(lhs, scale) => Self::reify_x(parent_bounds, prev_sibling, store, lhs)? / scale,
            Measurement::Store(name, x) => match dim_store {
                None => Err(LayoutError::new("can't use store for x: dim-store not applicable")),
                Some(dim_store) => {
                    let result = Self::reify_x(parent_bounds, prev_sibling, Some(dim_store), x).map_err(|err| err.add_store(name))?;
                    dim_store.insert(name, result);
                    result
                }
            }
            Measurement::Load(name) => match dim_store {
                None => Err(LayoutError::new("can't use load for x: dim-store not applicable")),
                Some(dim_store) => match dim_store.get(name) {
                    None => Err(LayoutError::new(format!("can't use load for x: no such dim {}", name))),
                    Some(result) => result
                }
            }
        })
    }

    fn reify_y(parent_bounds: &ParentBounds, prev_sibling: &PrevSiblingDim, dim_store: Option<&mut HashMap<&'static str, f32>>, y: &Measurement) -> LayoutResult<'_, f32> {
        Ok(match y {
            Measurement::Zero => 0f32,
            Measurement::Prev => match prev_sibling {
                PrevSiblingDim::NotApplicable => panic!("can't use prev for y: not applicable"),
                PrevSiblingDim::FirstChild => 0f32,
                PrevSiblingDim::Some(prev_sibling_dim) => prev_sibling_dim
            },
            Measurement::Units(y) => y,
            Measurement::Pixels(y) => y / parent_bounds.column_size.height,
            Measurement::Fraction(y) => match parent_bounds.bounding_box.height {
                None => Err(LayoutError::new("can't use fraction for y: parent height not known")),
                Some(height) => y * height
            },
            Measurement::Add(lhs, rhs) => Self::reify_y(parent_bounds, prev_sibling, store, lhs)? + Self::reify_y(parent_bounds, prev_sibling, store, rhs),
            Measurement::Sub(lhs, rhs) => Self::reify_y(parent_bounds, prev_sibling, store, lhs)? - Self::reify_y(parent_bounds, prev_sibling, store, rhs),
            Measurement::Mul(lhs, scale) => Self::reify_y(parent_bounds, prev_sibling, store, lhs)? * scale,
            Measurement::Div(lhs, scale) => Self::reify_y(parent_bounds, prev_sibling, store, lhs)? / scale,
            Measurement::Store(name, y) => match dim_store {
                None => Err(LayoutError::new("can't use store for y: dim-store not applicable")),
                Some(dim_store) => {
                    let result = Self::reify_y(parent_bounds, prev_sibling, Some(dim_store), y).map_err(|err| err.add_store(name))?;
                    dim_store.insert(name, result);
                    result
                }
            }
            Measurement::Load(name) => match dim_store {
                None => Err(LayoutError::new("can't use load for y: dim-store not applicable")),
                Some(dim_store) => match dim_store.get(name) {
                    None => Err(LayoutError::new(format!("can't use load for y: no such dim {}", name))),
                    Some(result) => result
                }
            }
        })
    }

    fn apply_layout_x(parent_bounds: &ParentBounds, prev_sibling: Option<&Rectangle>, layout: LayoutPosition1D, reified: f32) -> LayoutResult<'_, f32> {
        Ok(match layout {
            LayoutPosition1D::GlobalAbsolute => reified,
            LayoutPosition1D::LocalAbsolute => reified + parent_bounds.bounding_box.x,
            LayoutPosition1D::Relative => match parent_bounds.sub_layout.direction {
                LayoutDirection::Horizontal => match prev_sibling {
                    None => reified  + parent_bounds.bounding_box.left().map_err(|err| err.add_dimension("parent.left"))?,
                    Some(prev_sibling) => {
                        // Yes, we do want to reify the parent's sublayout with it's own bounds
                        let gap = Self::reify_x(parent_bounds, &PrevSiblingDim::NotApplicable, None, parent.sublayout.gap).map_err(|err| err.add_dimension("parent.gap"))?;
                        reified + prev_sibling.right + gap
                    }
                },
                LayoutDirection::Vertical => reified + parent_bounds.bounding_box.x,
                LayoutDirection::Overlap => reified + parent_bounds.bounding_box.x
            }
        })
    }

    fn apply_layout_y(parent_bounds: &ParentBounds, prev_sibling: Option<&Rectangle>, layout: LayoutPosition1D, reified: f32) -> LayoutResult<'_, f32> {
        Ok(match layout {
            LayoutPosition1D::GlobalAbsolute => reified,
            LayoutPosition1D::LocalAbsolute => reified + parent_bounds.bounding_box.y,
            LayoutPosition1D::Relative => match parent_bounds.sub_layout.direction {
                LayoutDirection::Horizontal => reified + parent_bounds.bounding_box.y,
                LayoutDirection::Vertical => match prev_sibling {
                    None => reified + parent_bounds.bounding_box.top().map_err(|err| err.add_dimension("parent.top"))?,
                    Some(prev_sibling) => {
                        // Yes, we do want to reify the parent's sublayout with it's own bounds
                        let gap = Self::reify_y(parent_bounds, &PrevSiblingDim::NotApplicable, None, parent.sublayout.gap).map_err(|err| err.add_dimension("parent.gap"))?;
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

impl BoundingBox {
    pub fn left(&self) -> LayoutResult<'_, f32> {
        if self.anchor_x == 0f32 {
            Ok(self.x)
        } else if let Some(width) = self.width {
            Ok(self.x - (self.anchor_x * width))
        } else {
            Err(LayoutError::new("bad layout: bounds not anchored at left with no width, so we don't know where its left is"))
        }
    }

    pub fn top(&self) -> LayoutResult<'_, f32> {
        if self.anchor_y == 0f32 {
            Ok(self.y)
        } else if let Some(height) = self.height {
            Ok(self.y - (self.anchor_y * height))
        } else {
            Err(LayoutError::new("bad layout: bounds not anchored at top with no height, so we don't know where its top is"))
        }
    }

    pub fn right(&self) -> LayoutResult<'_, f32> {
        if self.anchor_x == 1f32 {
            Ok(self.x)
        } else if let Some(width) = self.width {
            Ok(self.x + width - (self.anchor_x * width))
        } else {
            Err(LayoutError::new("bad layout: bounds not anchored at right with no width, so we don't know where its right is"))
        }
    }

    pub fn bottom(&self) -> LayoutResult<'_, f32> {
        if self.anchor_y == 1f32 {
            Ok(self.y)
        } else if let Some(height) = self.height {
            Ok(self.y + height - (self.anchor_y * height))
        } else {
            Err(LayoutError::new("bad layout: bounds not anchored at bottom with no height, so we don't know where its bottom is"))
        }
    }

    pub fn as_rectangle(&self) -> LayoutResult<'_, Rectangle> {
        if self.width.is_some() && self.height.is_some() {
            Ok(Rectangle {
                left: self.left().unwrap(),
                top: self.top().unwrap(),
                right: self.right().unwrap(),
                bottom: self.bottom().unwrap()
            })
        } else {
            Err(LayoutError::new("can't convert bounds into rectangle because there is no size"))
        }
    }
}

impl Rectangle {
    pub fn width(&self) -> f32 {
        self.right - self.left
    }

    pub fn height(&self) -> f32 {
        self.bottom + self.top
    }

    pub fn union(lhs: &Option<Rectangle>, rhs: &Option<Rectangle>) -> Option<Rectangle> {
        match (lhs, rhs) {
            (None, None) => None,
            (None, Some(rhs)) => Some(rhs.clone()),
            (Some(lhs), None) => Some(lhs.clone()),
            (Some(lhs), Some(rhs)) => Some(Rectangle {
                left: lhs.left.min(rhs.left),
                top: lhs.top.min(rhs.top),
                right: lhs.right.max(rhs.right),
                bottom: lhs.bottom.max(rhs.bottom)
            })
        }
    }

    pub fn intersection(lhs: &Option<Rectangle>, rhs: &Option<Rectangle>) -> Option<Rectangle> {
        match (lhs, rhs) {
            (None, None) => None,
            (None, Some(rhs)) => Some(rhs.clone()),
            (Some(lhs), None) => Some(lhs.clone()),
            (Some(lhs), Some(rhs)) => Rectangle {
                left: lhs.left.max(rhs.left),
                top: lhs.top.max(rhs.top),
                right: lhs.right.min(rhs.right),
                bottom: lhs.bottom.min(rhs.bottom)
            }.none_if_negative()
        }
    }

    /// Converts into null rectangle if any dimensions are negative
    fn none_if_negative(self: Rectangle) -> Option<Rectangle> {
        if self.left <= self.right && self.top <= self.bottom {
            Some(self)
        } else {
            None
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