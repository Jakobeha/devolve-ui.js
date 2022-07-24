//! Logs views and renders. Wraps a `RenderEngine` to intercept `render` calls and do logging then.

use std::collections::HashMap;
use std::fmt::Debug;
use std::io;
use crate::logging::common::{GenericLogger, LogStart};
use crate::renderer::render::{VRender, VRenderLayer};
use crate::view::layout::geom::Rectangle;
use crate::view::layout::parent_bounds::ParentBounds;
use crate::view::view::{VView, VViewData};
#[cfg(feature = "logging")]
use serde::{Serialize, Deserialize};
use crate::component::node::{NodeId, VComponentAndView, VNode};
use crate::component::path::VComponentKey;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RenderLogEntry<ViewData: VViewData, RenderLayer> {
    StartRendering,
    StopRendering,
    WriteRender(LoggedRenderTree<ViewData, RenderLayer>),
    Clear,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggedRender<RenderLayer> {
    parent_bounds: ParentBounds,
    prev_sibling_rect: Option<Rectangle>,
    render: VRender<RenderLayer>,
    was_cached: bool
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggedRenderView<ViewData: VViewData, RenderLayer> {
    pub view: Box<VView<ViewData>>,

    pub component_key: VComponentKey,
    pub parent_id: NodeId,
    pub prev_sibling_id: NodeId,

    pub render: LoggedRender<RenderLayer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoggedRenderTreeChild<ViewData: VViewData, RenderLayer> {
    Found(LoggedRenderTree<ViewData, RenderLayer>),
    Lost
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggedRenderTree<ViewData: VViewData, RenderLayer> {
    pub view: LoggedRenderView<ViewData, RenderLayer>,
    pub children: Vec<LoggedRenderTreeChild<ViewData, RenderLayer>>
}

pub struct RenderLoggerImpl<ViewData: VViewData, RenderLayer> {
    logger: GenericLogger<RenderLogEntry<ViewData, RenderLayer>>,

    view_map: HashMap<NodeId, LoggedRenderView<ViewData, RenderLayer>>,
    component_id_2_last_view_id: HashMap<NodeId, NodeId>,
    prev_sibling_id_map: HashMap<NodeId, NodeId>,
    last_id: NodeId
}

pub trait RenderLogger {
    type ViewData: VViewData;
    type RenderLayer;

    fn log_start_rendering(&mut self);
    fn log_stop_rendering(&mut self);

    fn log_write_render(&mut self) where Self::RenderLayer: VRenderLayer;
    fn log_clear(&mut self);
    fn log_render_view(
        &mut self,
        c_and_view: VComponentAndView<'_, Self::ViewData>,
        parent_id: NodeId,
        parent_bounds: &ParentBounds,
        prev_sibling_rect: Option<&Rectangle>,
        render: &VRender<Self::RenderLayer>,
        is_cached: bool
    ) where Self::RenderLayer: VRenderLayer;
}


impl <ViewData: VViewData + Serialize + Debug + Clone, RenderLayer: Serialize + Debug> RenderLoggerImpl<ViewData, RenderLayer> {
    pub(in crate::core) fn try_new(args: &LogStart) -> io::Result<Self> {
        Ok(RenderLoggerImpl {
            logger: GenericLogger::new(args, "renders")?,

            view_map: HashMap::new(),
            component_id_2_last_view_id: HashMap::new(),
            prev_sibling_id_map: HashMap::new(),
            last_id: NodeId::NULL
        })
    }

    fn log(&mut self, entry: RenderLogEntry<ViewData, RenderLayer>) {
        self.logger.log(entry)
    }

    fn last_view_of_component_id(&self, id: &NodeId) -> NodeId {
        self.component_id_2_last_view_id.get(id).copied().unwrap_or(NodeId::NULL)
    }

    fn drain_and_collapse_renders(&mut self, id: &NodeId) -> Option<LoggedRenderTree<ViewData, RenderLayer>> {
        let view = self.view_map.remove(id)?;
        let mut children = Vec::new();

        if let Some((view_children, _)) = view.view.d.children() {
            for view_child in view_children {
                let view_id = match view_child {
                    // Last id = root when we are done rendering (may be null if cached, that's ok, we'll fallthrough to LoggedRenderTreeChild::Lost)
                    VNode::Component { id, .. } => self.last_view_of_component_id(&id),
                    VNode::View(view_child) => view_child.id()
                };
                if let Some(child) = self.drain_and_collapse_renders(&view_id) {
                    children.push(LoggedRenderTreeChild::Found(child))
                } else {
                    children.push(LoggedRenderTreeChild::Lost)
                }
            }
        }

        Some(LoggedRenderTree {
            view,
            children
        })
    }
}

impl <ViewData: VViewData + Serialize + Debug + Clone, RenderLayer: Serialize + Debug> RenderLogger for RenderLoggerImpl<ViewData, RenderLayer> {
    type ViewData = ViewData;
    type RenderLayer = RenderLayer;

    fn log_start_rendering(&mut self) {
        self.log(RenderLogEntry::StartRendering);
    }

    fn log_stop_rendering(&mut self) {
        self.log(RenderLogEntry::StopRendering);
    }

    fn log_write_render(&mut self) where RenderLayer: VRenderLayer {
        // Last id = root when we are done rendering
        assert_ne!(self.last_id, NodeId::NULL, "no views rendered in one render, didn't expect that, need to handle");
        let last_id = self.last_id;
        let logged_render = self.drain_and_collapse_renders(&last_id).expect("no view for last_id, how?");

        self.view_map.clear();
        self.prev_sibling_id_map.clear();
        self.component_id_2_last_view_id.clear();
        self.last_id = NodeId::NULL;

        self.log(RenderLogEntry::WriteRender(logged_render));
    }

    fn log_clear(&mut self) {
        self.log(RenderLogEntry::Clear);
    }

    fn log_render_view(
        &mut self,
        (c, view): VComponentAndView<'_, ViewData>,
        parent_id: NodeId,
        parent_bounds: &ParentBounds,
        prev_sibling_rect: Option<&Rectangle>,
        render: &VRender<RenderLayer>,
        is_cached: bool
    ) where RenderLayer: VRenderLayer {
        let prev_sibling_id = self.prev_sibling_id_map.insert(parent_id, view.id()).unwrap_or(NodeId::NULL);
        let should_be_none = self.view_map.insert(view.id(), LoggedRenderView {
            view: view.clone(),

            component_key: *c.key(),
            parent_id,
            prev_sibling_id,

            render: LoggedRender {
                parent_bounds: parent_bounds.clone(),
                prev_sibling_rect: prev_sibling_rect.cloned(),

                render: render.clone(),
                was_cached: is_cached
            }
        });
        assert!(should_be_none.is_none(), "view with same id logged twice in one render: {}", view.id());
        // May get overwritten, that's ok and we discard old value...
        self.component_id_2_last_view_id.insert(c.id(), view.id());
        assert!(should_be_none.is_none(), "view with same id logged twice in one render: {}", view.id());
        // ...it's the exact same for replacing this
        self.last_id = view.id();
    }
}