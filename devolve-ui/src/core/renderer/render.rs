use std::collections::{btree_map, BTreeMap};
use crate::core::view::layout::geom::Rectangle;

pub struct VRender<Layer> {
    layers: BTreeMap<f32, Layer>,
    rect: Option<Rectangle>
}

impl VRender<Layer> {
    fn iter(&self) -> btree_map::Values<f32, Layer> {
        self.layers.values()
    }

    fn iter_mut(&mut self) -> btree_map::ValuesMut<f32, Layer> {
        self.layers.values_mut()
    }
}

impl <Layer> IntoIterator for VRender<Layer> {
    type Item = Layer;
    type IntoIter = btree_map::IntoValues<f32, Layer>;

    fn into_iter(self) -> Self::IntoIter {
        self.layers.into_values()
    }
}