//! Data which represents a final render, which will be drawn to screen.
use std::collections::{btree_map, BTreeMap};
use float_ord::FloatOrd;
use crate::core::view::layout::geom::Rectangle;

/// An individual render layer: what gets rendered from a view.
/// But actually a view renders into a `VRender` which is a stack of these layers with different
/// z-positions: `VRenderLayer` doesn't have to worry about z-positions or rect.
/// What type of `VRenderLayer` depends on the platform you're rendering to, e.g.
/// in `TuiViewData` it's just a 2d matrix of characters.
pub trait VRenderLayer : Clone {
    /// Clip out of bounds of `rect`.
    fn clip(&mut self, rect: &Rectangle);
}

/// An ordered stack of render layers each with a z-position.
/// It also has `rect` as a fast-access to the bounds of all of these renders.
#[derive(Debug, Clone)]
pub struct VRender<Layer> {
    layers: BTreeMap<FloatOrd<f64>, Layer>,
    rect: Option<Rectangle>
}

impl <Layer> VRender<Layer> {
    /// Create an empty render with no layers and null (`None`) rect.
    pub fn new() -> VRender<Layer> {
        VRender {
            layers: BTreeMap::new(),
            rect: None
        }
    }

    /// Add a render layer to the renderer. You must also provide it's z-position and bounds.
    pub fn insert(&mut self, z: f64, rect: Option<&Rectangle>, layer: Layer) {
        self.layers.insert(FloatOrd(z), layer);
        self.rect = Rectangle::union(self.rect(), rect);
    }

    /// Extend the render's bounds to include `rect` without actually modifying the layers.
    pub fn extend(&mut self, rect: Option<&Rectangle>) {
        self.rect = Rectangle::union(self.rect(), rect);
    }

    /// Add all of the layers of `other`. And drain it, as this uses `append`, and that's why it comsumes `other`.
    pub fn merge(&mut self, mut other: VRender<Layer>) {
        self.layers.append(&mut other.layers);
        self.rect = Rectangle::union(self.rect(), other.rect());
    }

    /// Render's outer bounds. May be even larger than the layers if we use `extend`.
    pub fn rect(&self) -> Option<&Rectangle> {
        self.rect.as_ref()
    }

    /// Iterate through the render's layers, from bottom to top.
    pub fn iter(&self) -> btree_map::Values<FloatOrd<f64>, Layer> {
        self.layers.values()
    }

    /// Iterate through the render's layers, from bottom to top.
    pub fn iter_mut(&mut self) -> btree_map::ValuesMut<FloatOrd<f64>, Layer> {
        self.layers.values_mut()
    }
}

impl <Layer: VRenderLayer> VRender<Layer> {
    /// Set the render's bounds to `rect. Clips if outside the bounds, extends if inside.
    pub fn clip_and_extend(&mut self, rect: Option<&Rectangle>) {
        match rect {
            None => self.layers.clear(),
            Some(rect) => for layer in self.iter_mut() {
                layer.clip(rect);
            }
        }
        self.rect = rect.cloned();
    }

    /// Clips any part of the render outside of bounds: clips layers, and sets rect to the intersection.
    pub fn clip(&mut self, rect: Option<&Rectangle>) {
        match rect {
            None => self.layers.clear(),
            Some(rect) => for layer in self.iter_mut() {
                layer.clip(rect);
            }
        }
        self.rect = Rectangle::intersection(self.rect(), rect);
    }
}

impl <Layer> IntoIterator for VRender<Layer> {
    type Item = Layer;
    type IntoIter = btree_map::IntoValues<FloatOrd<f64>, Layer>;

    /// Consume the render and iterate its layers, from bottom to top
    fn into_iter(self) -> Self::IntoIter {
        self.layers.into_values()
    }
}