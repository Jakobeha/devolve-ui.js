//! A `RenderEngine` renders to a specific platform. For example, the `TuiRenderEngine` renders to a TUI.
//!
//! Currently `TuiRenderEngine` is the only built-in one; however you can also create a render engine
//! to render to a desktop platform, web, or anywhere else.
//!
//! ## View Data
//! Each `RenderEngine` type has a close dependency to its `ViewData` type. `ViewData` is (generally an enum)
//! for all possible types of views: e.g. text, images, borders. If a render engine allows
//! e.g. platform-specific UI widgets or HTML components or filters, these would be representable in
//! `ViewData` enum variants.

#[cfg(feature = "input")]
use bitflags::bitflags;
use crate::core::view::layout::geom::{BoundingBox, Size};
use crate::core::view::layout::parent_bounds::ParentBounds;
use crate::core::view::view::{VView, VViewData};
use crate::core::renderer::render::VRender;
#[cfg(feature = "time")]
use crate::core::renderer::renderer::RendererViewForEngineInTick;
use crate::core::renderer::traceback::RenderTraceback;
use crate::core::view::layout::err::LayoutError;

#[cfg(feature = "input")]
bitflags! {
    /// Bitmask of different input types
    pub struct InputListeners: u8 {
        const KEYS = 0b0000_0001;
        const MOUSE = 0b0000_0010;
        /// Resize window or column
        const RESIZE = 0b0000_0100;
    }
}

/// See module-level documentation
pub trait RenderEngine {
    /// View data for the render engine. Different render engines render different views.
    /// For example, an HTML render engine may have view-data enum variants representing HTML elements.
    type ViewData: VViewData;
    /// Representation of the rendered output. e.g. it's a matrix of characters in `TuiRenderEngine`.
    type RenderLayer;

    /// Get the dimensions of the root window, "column size" (text size), and other info.
    fn get_root_dimensions(&self) -> ParentBounds;
    /// Register the callback for when the view resizes. May be deprecated as there are other ways to send resize events.
    fn on_resize(&mut self, callback: Box<dyn Fn() + Send + Sync>);

    /// Called before the `Renderer` is made visible.
    fn start_rendering(&mut self);
    /// Called before the `Renderer` is made invisible.
    fn stop_rendering(&mut self);
    /// Called each time the `Renderer` needs to draw the final render to the screen.
    /// Either `start_rendering` or `clear` is guaranteed to be called right before this.
    fn write_render(&mut self, batch: VRender<Self::RenderLayer>);
    /// Called each time the `Renderer` needs to clear for a next renderer or `stop_rendering`.
    /// Either `stop_rendering` or `write_render` is guaranteed to be called right after this.
    fn clear(&mut self);

    /// Render a single view.
    /// Any of the view's children are already rendered in `rendererd_children`, but you
    /// can do whatever you want to this render (e.g. apply a filter to it).
    fn make_render(
        &self,
        bounds: &BoundingBox,
        column_size: &Size,
        view: &Box<VView<Self::ViewData>>,
        rendered_children: VRender<Self::RenderLayer>,
        traceback: &RenderTraceback<Self::ViewData>,
    ) -> Result<VRender<Self::RenderLayer>, LayoutError>;

    /// Called each tick when the renderer `is_running`. This is where you can send inputs to the renderer.
    /// However, don't send any inputs unless `update_input_listeners` was called with corresponding
    /// listeners set.
    #[cfg(feature = "time")]
    fn tick<Root: RenderEngine>(&mut self, engine: RendererViewForEngineInTick<'_, Root>) where Self: Sized;

    /// Called each time the renderer registers or unregisters input listeners.
    /// You should not be sending inputs to the renderer except those where `input_listeners` is set.
    /// Otherwise you will have issues e.g. in tests or anywhere else where the user explicitly
    /// does not want your events.
    #[cfg(feature = "input")]
    fn update_input_listeners(&mut self, input_listeners: InputListeners);
}