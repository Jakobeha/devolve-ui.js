#[cfg(feature = "input")]
use bitflags::bitflags;
use crate::core::view::layout::geom::{BoundingBox, Size};
use crate::core::view::layout::parent_bounds::ParentBounds;
use crate::core::view::view::{VView, VViewData};
use crate::core::renderer::render::VRender;
#[cfg(feature = "time")]
use crate::core::renderer::renderer::RendererViewForEngineInTick;
use crate::core::view::layout::err::LayoutError;

#[cfg(feature = "input")]
bitflags! {
    pub struct InputListeners: u8 {
        const KEYS = 0b0000_0001;
        const MOUSE = 0b0000_0010;
        const RESIZE = 0b0000_0100;
    }
}

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

    #[cfg(feature = "time")]
    fn tick(&mut self, engine: RendererViewForEngineInTick<'_, Self>);

    #[cfg(feature = "input")]
    fn update_input_listeners(&mut self, input_listeners: InputListeners);
}