use std::collections::HashMap;

#[derive(Clone)]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub anchor_x: f32,
    pub anchor_y: f32,
}

#[derive(Clone)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

#[derive(Clone)]
pub struct Rectangle {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Copy)]
pub enum LayoutDirection {
    Horizontal,
    Vertical,
    Overlap
}

#[derive(Clone)]
pub struct SubLayout {
    pub direction: LayoutDirection,
    pub gap: Option<Measurement>
}

#[derive(Clone)]
pub struct DimMap<T> {
    pub x: T,
    pub y: T,
    pub width: T,
    pub height: T,
}

pub type DimsStore = DimMap<HashMap<&'static str, f32>>;

#[derive(Clone)]
pub struct ParentBounds {
    pub bounding_box: BoundingBox,
    pub sub_layout: SubLayout,
    pub column_size: Size,
    pub store: DimsStore
}

#[derive(Clone, Copy)]
pub enum LayoutPosition1D {
    GlobalAbsolute,
    LocalAbsolute,
    Relative
}

#[derive(Clone)]
pub struct LayoutPosition {
    x: LayoutPosition1D,
    y: LayoutPosition1D,
}

#[derive(Clone)]
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

#[derive(Clone)]
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
    pub fn resolve(&self, parent_bounds: &ParentBounds, prev_sibling: Option<&Rectangle>) -> (BoundingBox, DimsStore) {
        let mut store = parent_bounds.store.clone();
        let bounding_box = BoundingBox {
            x: Self::apply_layout_x(parent_bounds, prev_sibling, self.layout.x, Self::reify_x(parent_bounds, PrevSibling::NotApplicable, Some(&mut store.x), &self.x)),
            y: Self::apply_layout_y(parent_bounds, prev_sibling, self.layout.y, Self::reify_y(parent_bounds, PrevSibling::NotApplicable, Some(&mut store.y), &self.y)),
            z: self.z + parent_bounds.bounding_box.z,
            anchor_x: self.anchor_x,
            anchor_y: self.anchor_y,
            width: self.width.map(|width| Self::reify_x(parent_bounds, prev_sibling.map(|r| r.width).into(), Some(&mut store.width), &width)),
            height: self.height.map(|height| Self::reify_y(parent_bounds, prev_sibling.map(|r| r.height).inti(), Some(&mut store.height), &height))
        };
        (bounding_box, store)
    }

    fn reify_x(parent_bounds: &ParentBounds, prev_sibling: &PrevSiblingDim, dim_store: Option<&mut HashMap<&'static str, f32>>, x: &Measurement) -> f32 {
        match x {
            Measurement::Zero => 0f32,
            Measurement::Prev => match prev_sibling {
                PrevSiblingDim::NotApplicable => panic!("can't use prev for x: not applicable"),
                PrevSiblingDim::FirstChild => 0f32,
                PrevSiblingDim::Some(prev_sibling_dim) => prev_sibling_dim
            },
            Measurement::Units(x) => x,
            Measurement::Pixels(x) => x / parent_bounds.column_size.width,
            Measurement::Fraction(x) => match parent_bounds.bounding_box.width {
                None => panic!("can't use fraction for x: parent width not known"),
                Some(width) => x * width,
            },
            Measurement::Add(lhs, rhs) => Self::reify_x(parent_bounds, prev_sibling, store, lhs) + Self::reify_x(parent_bounds, prev_sibling, store, rhs),
            Measurement::Sub(lhs, rhs) => Self::reify_x(parent_bounds, prev_sibling, store, lhs) - Self::reify_x(parent_bounds, prev_sibling, store, rhs),
            Measurement::Mul(lhs, scale) => Self::reify_x(parent_bounds, prev_sibling, store, lhs) * scale,
            Measurement::Div(lhs, scale) => Self::reify_x(parent_bounds, prev_sibling, store, lhs) / scale,
            Measurement::Store(name, x) => match dim_store {
                None => panic!("can't use store for x: dim-store not applicable"),
                Some(dim_store) => {
                    let result = Self::reify_x(parent_bounds, prev_sibling, Some(dim_store), x);
                    dim_store.insert(name, result);
                    result
                }
            }
            Measurement::Load(name) => match dim_store {
                None => panic!("can't use load for x: dim-store not applicable"),
                Some(dim_store) => match dim_store.get(name) {
                    None => panic!("can't use load for x: no such dim {}", name),
                    Some(result) => result
                }
            }
        }
    }

    fn reify_y(parent_bounds: &ParentBounds, prev_sibling: &PrevSiblingDim, dim_store: Option<&mut HashMap<&'static str, f32>>, y: &Measurement) -> f32 {
        match y {
            Measurement::Zero => 0f32,
            Measurement::Prev => match prev_sibling {
                PrevSiblingDim::NotApplicable => panic!("can't use prev for y: not applicable"),
                PrevSiblingDim::FirstChild => 0f32,
                PrevSiblingDim::Some(prev_sibling_dim) => prev_sibling_dim
            },
            Measurement::Units(y) => y,
            Measurement::Pixels(y) => y / parent_bounds.column_size.height,
            Measurement::Fraction(y) => match parent_bounds.bounding_box.height {
                None => panic!("can't use fraction for x: parent width not known"),
                Some(height) => y * height,
            },
            Measurement::Add(lhs, rhs) => Self::reify_y(parent_bounds, prev_sibling, store, lhs) + Self::reify_y(parent_bounds, prev_sibling, store, rhs),
            Measurement::Sub(lhs, rhs) => Self::reify_y(parent_bounds, prev_sibling, store, lhs) - Self::reify_y(parent_bounds, prev_sibling, store, rhs),
            Measurement::Mul(lhs, scale) => Self::reify_y(parent_bounds, prev_sibling, store, lhs) * scale,
            Measurement::Div(lhs, scale) => Self::reify_y(parent_bounds, prev_sibling, store, lhs) / scale,
            Measurement::Store(name, y) => match dim_store {
                None => panic!("can't use store for x: dim-store not applicable"),
                Some(dim_store) => {
                    let result = Self::reify_y(parent_bounds, prev_sibling, Some(dim_store), y);
                    dim_store.insert(name, result);
                    result
                }
            }
            Measurement::Load(name) => match dim_store {
                None => panic!("can't use load for x: dim-store not applicable"),
                Some(dim_store) => match dim_store.get(name) {
                    None => panic!("can't use load for x: no such dim {}", name),
                    Some(result) => result
                }
            }
        }
    }

    fn apply_layout_x(parent_bounds: &ParentBounds, prev_sibling: Option<&Rectangle>, layout: LayoutPosition1D, reified: f32) -> f32 {
        match layout {
            LayoutPosition1D::GlobalAbsolute => reified,
            LayoutPosition1D::LocalAbsolute => reified + parent_bounds.bounding_box.x,
            LayoutPosition1D::Relative => match parent_bounds.sub_layout.direction {
                LayoutDirection::Horizontal => match prev_sibling {
                    None => reified + parent_bounds.bounding_box.left(),
                    Some(prev_sibling) => {
                        // Yes, we do want to reify the parent's sublayout with it's own bounds
                        let gap = Self::reify_x(parent_bounds, &PrevSiblingDim::NotApplicable, None, parent.sublayout.gap);
                        reified + prev_sibling.right() + gap
                    }
                },
                LayoutDirection::Vertical => reified + parent_bounds.bounding_box.x,
                LayoutDirection::Overlap => reified + parent_bounds.bounding_box.x
            }
        }
    }

    fn apply_layout_y(parent_bounds: &ParentBounds, prev_sibling: Option<&Rectangle>, layout: LayoutPosition1D, reified: f32) -> f32 {
        match layout {
            LayoutPosition1D::GlobalAbsolute => reified,
            LayoutPosition1D::LocalAbsolute => reified + parent_bounds.bounding_box.y,
            LayoutPosition1D::Relative => match parent_bounds.sub_layout.direction {
                LayoutDirection::Horizontal => reified + parent_bounds.bounding_box.y,
                LayoutDirection::Vertical => match prev_sibling {
                    None => reified + parent_bounds.bounding_box.top(),
                    Some(prev_sibling) => {
                        // Yes, we do want to reify the parent's sublayout with it's own bounds
                        let gap = Self::reify_y(parent_bounds, &PrevSiblingDim::NotApplicable, None, parent.sublayout.gap);
                        reified + prev_sibling.bottom() + gap
                    }
                },
                LayoutDirection::Overlap => reified + parent_bounds.bounding_box.y
            }
        }
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