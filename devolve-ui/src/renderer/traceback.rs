//! Render-specific backtrace info for debugging render issues. Currently only used for bounds.
//! This is passed around the render code so that when an issue arises, it gets logged along with the warning.
//!
//! This has a similar purpose as `logging`, but exists and works separately.
//! Both formats are for debugging incorrect renders, and both can be parsed by a computer or read directly.
//! This is mainly for logging errors and assertion failures, and also more readable as-is.
//! `logging` is mainly for general-purpose logs, and also more readable by programs.
//! `logging` is also for editing renders and seeing the changes faster, and for style purposes.
//! TODO: Add the ability for render assertions (e.g. assert that dimension is something)

use std::fmt::{Display, Formatter};
use std::marker::PhantomData;
use derivative::Derivative;
use crate::component::node::NodeId;
use crate::view::layout::bounds::Bounds;
use crate::view::layout::geom::{BoundingBox, Rectangle};
use crate::view::view::{VView, VViewData};

#[derive(Derivative)]
#[derivative(Debug(bound=""), Clone(bound=""))]
pub struct RenderTraceback<ViewData: VViewData + ?Sized>(Vec<RenderFrame<ViewData>>);

#[derive(Derivative)]
#[derivative(Debug(bound=""), Clone(bound=""))]
struct RenderFrame<ViewData: VViewData + ?Sized> {
    view: Option<RenderFrameViewData>,
    resolved_bounds: Option<BoundingBox>,
    prev_sibling_rect: Option<Rectangle>,
    phantom: PhantomData<ViewData>
}

#[derive(Debug, Clone)]
struct RenderFrameViewData {
    id: NodeId,
    bounds: Bounds
}

impl <ViewData: VViewData + ?Sized> RenderTraceback<ViewData> {
    pub fn root() -> Self {
        Self(vec![RenderFrame::root()])
    }

    pub fn child(&self, prev_sibling_rect: Option<&Rectangle>) -> Self {
        let mut child = self.clone();
        child.0.push(RenderFrame::child(prev_sibling_rect));
        child
    }

    fn top(&mut self) -> &mut RenderFrame<ViewData> {
        self.0.last_mut().unwrap()
    }

    pub fn add_view(&mut self, view: &Box<VView<ViewData>>) {
        self.top().add_view(view)
    }

    pub fn add_resolved_bounds(&mut self, bounding_box: &BoundingBox) {
        self.top().add_resolved_bounds(bounding_box)
    }
}

impl <ViewData: VViewData + ?Sized> RenderFrame<ViewData> {
    fn root() -> Self {
        RenderFrame {
            view: None,
            resolved_bounds: None,
            prev_sibling_rect: None,
            phantom: PhantomData
        }
    }

    fn child(prev_sibling_rect: Option<&Rectangle>) -> Self {
        RenderFrame {
            view: None,
            resolved_bounds: None,
            prev_sibling_rect: prev_sibling_rect.cloned(),
            phantom: PhantomData
        }
    }

    fn add_view(&mut self, view: &Box<VView<ViewData>>) {
        assert!(self.view.is_none(), "already added view");
        self.view = Some(RenderFrameViewData::from(view));
    }

    fn add_resolved_bounds(&mut self, bounding_box: &BoundingBox) {
        assert!(self.resolved_bounds.is_none(), "already added bounds");
        self.resolved_bounds = Some(bounding_box.clone());
    }
}

impl <ViewData: VViewData + ?Sized> From<&Box<VView<ViewData>>> for RenderFrameViewData {
    fn from(view: &Box<VView<ViewData>>) -> Self {
        RenderFrameViewData {
            id: view.id(),
            bounds: view.bounds.clone()
        }
    }
}

impl <ViewData: VViewData + ?Sized> Display for RenderTraceback<ViewData> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "  Traceback:\n")?;
        for frame in self.0.iter().rev() {
            write!(f, "    {}\n", frame)?;
        }
        Ok(())
    }
}

impl <ViewData: VViewData + ?Sized> Display for RenderFrame<ViewData> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.view {
            None => write!(f, "{:>8} {:>32}", "null", "null")?,
            Some(view) => write!(f, "{:>8} {:>32}", view.id, view.bounds)?
        }
        write!(f, " => ")?;
        match &self.resolved_bounds {
            None => write!(f, "{:>24}", "null")?,
            Some(resolved_bounds) => write!(f, "{:>24}", resolved_bounds)?
        }
        write!(f, " prev: ")?;
        match &self.prev_sibling_rect {
            None => write!(f, "{:>16}", "null")?,
            Some(prev_sibling_rect) => write!(f, "{:>16}", prev_sibling_rect)?
        }
        Ok(())
    }
}