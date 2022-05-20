use crate::core::view::layout::geom::{BoundingBox, Rectangle, Size};
use crate::core::view::layout::parent_bounds::ParentBounds;
use crate::core::view::view::{VView, VViewData};
use crate::core::renderer::render::{VRender, VRenderLayer};

pub trait RenderEngine {
    type ViewData: VViewData;
    type RenderLayer: VRenderLayer;

    fn get_root_dimensions(&self) -> ParentBounds;
    fn on_resize(&mut self, callback: Box<dyn Fn() -> ()>);

    fn start_rendering(&mut self);
    fn stop_rendering(&mut self);
    fn write_render(&mut self, batch: VRender<RenderLayer>);
    fn clear(&mut self);

    fn make_render(&self, bounds: &BoundingBox, column_size: &Size, view: &Box<VView<ViewData>>, rendered_children: VRender<RenderLayer>) -> VRender<RenderLayer>;

    // fn use_input(handler: impl FnMut(Key) -> ()) -> dyn FnOnce() -> ();
}