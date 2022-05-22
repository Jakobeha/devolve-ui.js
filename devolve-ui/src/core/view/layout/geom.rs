use crate::core::view::layout::err::{LayoutError, LayoutResult};

#[derive(Debug, Clone, PartialEq)]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub anchor_x: f32,
    pub anchor_y: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rectangle {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl BoundingBox {
    pub fn left(&self) -> LayoutResult<f32> {
        if self.anchor_x == 0f32 {
            Ok(self.x)
        } else if let Some(width) = self.width {
            Ok(self.x - (self.anchor_x * width))
        } else {
            Err(LayoutError::new("bad layout: bounds not anchored at left with no width, so we don't know where its left is"))
        }
    }

    pub fn top(&self) -> LayoutResult<f32> {
        if self.anchor_y == 0f32 {
            Ok(self.y)
        } else if let Some(height) = self.height {
            Ok(self.y - (self.anchor_y * height))
        } else {
            Err(LayoutError::new("bad layout: bounds not anchored at top with no height, so we don't know where its top is"))
        }
    }

    pub fn right(&self) -> LayoutResult<f32> {
        if self.anchor_x == 1f32 {
            Ok(self.x)
        } else if let Some(width) = self.width {
            Ok(self.x + width - (self.anchor_x * width))
        } else {
            Err(LayoutError::new("bad layout: bounds not anchored at right with no width, so we don't know where its right is"))
        }
    }

    pub fn bottom(&self) -> LayoutResult<f32> {
        if self.anchor_y == 1f32 {
            Ok(self.y)
        } else if let Some(height) = self.height {
            Ok(self.y + height - (self.anchor_y * height))
        } else {
            Err(LayoutError::new("bad layout: bounds not anchored at bottom with no height, so we don't know where its bottom is"))
        }
    }

    pub fn with_default_size(&self, default_size: &Size) -> Self {
        BoundingBox {
            x: self.x,
            y: self.y,
            z: self.z,
            width: self.width.or(Some(default_size.width)),
            height: self.height.or(Some(default_size.height)),
            anchor_x: self.anchor_x,
            anchor_y: self.anchor_y
        }
    }

    pub fn as_rectangle(&self) -> LayoutResult<Rectangle> {
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

    pub fn as_rectangle_with_default_size(&self, default_size: &Size) -> Rectangle {
        self.with_default_size(default_size).as_rectangle().expect("as_rectangle_with_default_size: didn't expect a layout error was possible here")
    }
}

impl Rectangle {
    pub fn width(&self) -> f32 {
        self.right - self.left
    }

    pub fn height(&self) -> f32 {
        self.bottom + self.top
    }

    pub fn union(lhs: Option<&Rectangle>, rhs: Option<&Rectangle>) -> Option<Rectangle> {
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

    pub fn intersection(lhs: Option<&Rectangle>, rhs: Option<&Rectangle>) -> Option<Rectangle> {
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