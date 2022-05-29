use std::collections::{btree_map, BTreeMap};
use float_ord::FloatOrd;
use crate::core::view::layout::geom::Rectangle;

pub trait VRenderLayer : Clone {
    fn clip(&mut self, rect: &Rectangle);
}

#[derive(Debug, Clone)]
pub struct VRender<Layer> {
    layers: BTreeMap<FloatOrd<f64>, Layer>,
    rect: Option<Rectangle>
}

impl <Layer> VRender<Layer> {
    pub fn new() -> VRender<Layer> {
        VRender {
            layers: BTreeMap::new(),
            rect: None
        }
    }

    pub fn insert(&mut self, z: f64, rect: Option<&Rectangle>, layer: Layer) {
        self.layers.insert(FloatOrd(z), layer);
        self.rect = Rectangle::union(self.rect(), rect);
    }
    pub fn extend(&mut self, rect: Option<&Rectangle>) {
        self.rect = Rectangle::union(self.rect(), rect);
    }

    pub fn merge(&mut self, mut other: VRender<Layer>) {
        self.layers.append(&mut other.layers);
        self.rect = Rectangle::union(self.rect(), other.rect());
    }

    pub fn rect(&self) -> Option<&Rectangle> {
        self.rect.as_ref()
    }

    pub fn iter(&self) -> btree_map::Values<FloatOrd<f64>, Layer> {
        self.layers.values()
    }

    pub fn iter_mut(&mut self) -> btree_map::ValuesMut<FloatOrd<f64>, Layer> {
        self.layers.values_mut()
    }
}

impl <Layer: VRenderLayer> VRender<Layer> {
    pub fn clip_and_extend(&mut self, rect: Option<&Rectangle>) {
        match rect {
            None => self.layers.clear(),
            Some(rect) => for layer in self.iter_mut() {
                layer.clip(rect);
            }
        }
        self.rect = rect.cloned();
    }

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

    fn into_iter(self) -> Self::IntoIter {
        self.layers.into_values()
    }
}