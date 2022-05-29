use std::borrow::Cow;
use crate::core::component::node::VNode;
use crate::core::view::layout::bounds::{Bounds, LayoutPosition, Measurement};
use crate::core::view::view::{VView, VViewData};

/// Arguments for a less verbose constructor
#[derive(Debug)]
pub struct VViewConstrArgs {
    pub layout: LayoutPosition,
    pub x: Measurement,
    pub y: Measurement,
    pub z: i32,
    pub width: Option<Measurement>,
    pub height: Option<Measurement>,
    pub anchor_x: f32,
    pub anchor_y: f32,
    pub is_visible: bool,
    pub key: Option<Cow<'static, str>>,
}

impl Default for VViewConstrArgs {
    fn default() -> Self {
        VViewConstrArgs {
            layout: Bounds::default().layout,
            x: Bounds::default().x,
            y: Bounds::default().y,
            z: Bounds::default().z,
            width: Bounds::default().width,
            height: Bounds::default().height,
            anchor_x: Bounds::default().anchor_x,
            anchor_y: Bounds::default().anchor_y,
            is_visible: true,
            key: None
        }
    }
}

/// Create a less verbose constructor for a specific type of view
pub fn constr_view<ViewData: VViewData>(
    view_args: VViewConstrArgs,
    view_data: ViewData
) -> VNode<ViewData> {
    VNode::View(Box::new(VView::new(
        Bounds {
            layout: view_args.layout,
            x: view_args.x,
            y: view_args.y,
            z: view_args.z,
            width: view_args.width,
            height: view_args.height,
            anchor_x: view_args.anchor_x,
            anchor_y: view_args.anchor_y
        },
        view_args.is_visible,
        view_args.key,
        view_data
    )))
}

/// Create a less verbose constructor for a specific type of view
pub macro make_view_constr($f:expr) {
    move |view_args: VViewConstrArgs, data_args: ConstrArgs| {
        VNode::View(Box::new(VView::new(
            Bounds {
                layout: view_args.layout,
                x: view_args.x,
                y: view_args.y,
                z: view_args.z,
                width: view_args.width,
                height: view_args.height,
                anchor_x: view_args.anchor_x,
                anchor_y: view_args.anchor_y
            },
            view_args.is_visible,
            view_args.key,
            f(data_args)
        )))
    }
}