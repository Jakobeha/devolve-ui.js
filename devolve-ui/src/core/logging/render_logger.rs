//! Logs views and renders. Wraps a `RenderEngine` to intercept `render` calls and do logging then.

use crate::core::logging::common::GenericLogger;
use crate::core::renderer::engine::{InputListeners, RenderEngine};
use crate::core::renderer::render::VRender;
use crate::core::renderer::renderer::RendererViewForEngineInTick;
use crate::core::view::layout::err::LayoutError;
use crate::core::view::layout::geom::{BoundingBox, Size};
use crate::core::view::layout::parent_bounds::ParentBounds;
use crate::core::view::view::{VView, VViewData};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RenderLogEntry<ViewData: VViewData, RenderLayer> {
    StartRendering,
    StopRendering,
    WriteRender(VRender<Self::RenderLayer>),
    Clear,
}

pub struct RenderLogger<Engine: RenderEngine> {
    engine: Engine,
    logger: GenericLogger<RenderLogEntry<Engine::ViewData, Engine::RenderLayer>>
}

impl <Engine: RenderEngine> RenderLogger<Engine> {
    fn log(&mut self, entry: RenderLogEntry<Engine::ViewData, Engine::RenderLayer>) {
        self.logger.log(entry)
    }
}

impl <Engine: RenderEngine> RenderEngine for RenderLogger<Engine> where Engine::RenderLayer: Clone {
    type ViewData = Engine::ViewData;
    // TODO: change RenderLayer?
    type RenderLayer = Engine::RenderLayer;

    fn get_root_dimensions(&self) -> ParentBounds {
        self.engine.get_root_dimensions()
    }

    fn on_resize(&mut self, callback: Box<dyn Fn() + Send + Sync>) {
        self.engine.on_resize(callback)
    }

    fn start_rendering(&mut self) {
        self.log(RenderLogEntry::StartRendering);
        self.engine.start_rendering()
    }

    fn stop_rendering(&mut self) {
        self.engine.stop_rendering();
        self.log(RenderLogEntry::StopRendering);
    }

    fn write_render(&mut self, batch: VRender<Self::RenderLayer>) {
        self.log(RenderLogEntry::WriteRender(batch.clone()));
        self.engine.write_render(batch);
    }

    fn clear(&mut self) {
        self.log(RenderLogEntry::Clear);
        self.engine.clear()
    }

    fn make_render(&self, bounds: &BoundingBox, column_size: &Size, view: &Box<VView<Self::ViewData>>, rendered_children: VRender<Self::RenderLayer>) -> Result<VRender<Self::RenderLayer>, LayoutError> {
        // TODO
        self.engine.make_render(bounds, column_size, view, rendered_children)
    }

    fn tick(&mut self, engine: RendererViewForEngineInTick<'_, Self>) where Self: Sized {
        self.engine.tick(engine)
    }

    fn update_input_listeners(&mut self, input_listeners: InputListeners) {
        self.engine.update_input_listeners(input_listeners)
    }
}