use crate::core::view::layout::geom::{BoundingBox, Rectangle, Size};
use crate::core::view::layout::parent_bounds::ParentBounds;
use crate::core::view::view::{VView, VViewData};
use crate::renderer::renderer::VRender;

pub trait RenderEngine {
    type ViewData: VViewData;
    type RenderLayer;

    fn get_root_dimensions(&self) -> ParentBounds;
    fn on_resize(&mut self, callback: Box<dyn FnMut(ParentBounds) -> ()>);

    fn start_rendering(&mut self);
    fn stop_rendering(&mut self);
    fn write_render(&mut self, batch: VRender<RenderLayer>);
    fn clear(&mut self);

    fn clip(&self, layer: &mut RenderLayer, clip_rect: &Rectangle, column_size: &Size);
    fn make_render(&self, bounds: &BoundingBox, column_size: &Size, view: &VView<ViewData>) -> RenderLayer;
    // fn text(&self, bounds: &BoundingBox, column_size: &Size, wrap_mode: &Option<WrapMode>, color: &Option<Color>, text: &str): Layer;
    // fn solid_color(&self, bounds: &BoundingBox, column_size: &Size);

    // fn use_input(handler: impl FnMut(Key) -> ()) -> dyn FnOnce() -> ();
}