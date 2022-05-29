use crate::core::view::layout::geom::{BoundingBox, Size};
use crate::core::view::layout::parent_bounds::ParentBounds;
use crate::core::view::view::{VView, VViewData};
use crate::core::renderer::render::VRender;
use crate::core::view::layout::err::LayoutError;

pub trait RenderEngine {
    type ViewData: VViewData;
    type RenderLayer;

    fn get_root_dimensions(&self) -> ParentBounds;
    fn on_resize(&mut self, callback: Box<dyn Fn() + Send + Sync>);

    fn start_rendering(&mut self);
    fn stop_rendering(&mut self);
    fn write_render(&mut self, batch: VRender<Self::RenderLayer>);
    fn clear(&mut self);

    fn make_render(
        &self,
        bounds: &BoundingBox,
        column_size: &Size,
        view: &Box<VView<Self::ViewData>>,
        rendered_children: VRender<Self::RenderLayer>
    ) -> Result<VRender<Self::RenderLayer>, LayoutError>;

    // fn use_input(handler: impl FnMut(Key) -> ()) -> dyn FnOnce() -> ();
}