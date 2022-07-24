//! Terse constructors for `TuiViewData` views.

#![allow(non_upper_case_globals)]

use devolve_ui::component::node::VNode;
use devolve_ui::view::color::Color;
#[allow(unused_imports)] // Needed for IntelliJ macro expansion
use devolve_ui::view::constr::{_make_view, constr_view, make_view, VViewConstrArgs};
use devolve_ui::view::layout::measurement::Measurement;
use devolve_ui::view::layout::parent_bounds::{LayoutDirection, SubLayout};
use crate::view_data::attrs::{BorderStyle, DividerDirection, DividerStyle, TextWrapMode};
#[cfg(feature = "images")]
use crate::view_data::terminal_image::{HandleAspectRatio, Source};
use crate::view_data::tui::{HasTuiViewData, TuiViewData};

pub struct BoxConstrArgs {
    pub gap: Measurement,
    pub clip: bool,
    pub extend: bool
}

pub type Vvw1 = VViewConstrArgs;
pub type Hbx1 = BoxConstrArgs;
pub type Vbx1 = BoxConstrArgs;
pub type Zbx1 = BoxConstrArgs;
pub type Txt1 = TextConstrArgs;
pub type Bor1 = BorderConstrArgs;
pub type Hdv1 = HDividerConstrArgs;
pub type Vdv1 = VDividerConstrArgs;
#[cfg(feature = "images")]
pub type Src1 = SourceConstrArgs;

impl Default for BoxConstrArgs {
    fn default() -> Self {
        BoxConstrArgs {
            gap: Measurement::default(),
            clip: false,
            extend: false
        }
    }
}

macro _box2(($d:tt) @ $name:ident, $layout_direction: expr) {
    pub fn $name<ViewData: HasTuiViewData>(view_args: VViewConstrArgs, data_args: BoxConstrArgs, children: Vec<VNode<ViewData>>) -> VNode<ViewData> {
        constr_view(view_args, ViewData::tui_box(
            children,
            $crate::view_data::tui::TuiBoxAttrs {
                sub_layout: SubLayout {
                    direction: $layout_direction,
                    gap: data_args.gap
                },
                clip: data_args.clip,
                extend: data_args.extend
            }
        ))
    }

    pub macro $name({ $d( $d view_field:ident : $d view_value:expr ),* }, { $d( $d data_field:ident: $d data_value:expr ),* }, $d children:expr) {
        $name(VViewConstrArgs {
            $d( $d view_field : $d view_value, )*
            ..VViewConstrArgs::default()
        }, BoxConstrArgs {
            $d( $d data_field : $d data_value, )*
            ..BoxConstrArgs::default()
        }, $d children)
    }
}

macro _box($name:ident, $layout_direction:expr) {
    _box2!(($) @ $name, $layout_direction);
}

_box!(hbox, LayoutDirection::Horizontal);
_box!(vbox, LayoutDirection::Vertical);
_box!(zbox, LayoutDirection::Overlap);

pub fn ce_zbox<ViewData: HasTuiViewData>(view_args: VViewConstrArgs, children: Vec<VNode<ViewData>>) -> VNode<ViewData> {
    zbox(view_args, BoxConstrArgs {
        clip: true,
        extend: true,
        ..BoxConstrArgs::default()
    }, children)
}

pub macro ce_zbox({ $( $view_field:ident: $view_value:expr ),* }, $children:expr) {
    ce_zbox(VViewConstrArgs {
        $( $view_field : $view_value, )*
        ..VViewConstrArgs::default()
    }, $children)
}

make_view!(pub text, TextConstrArgs {
    color: Option<Color>,
    wrap_mode: TextWrapMode
} [ text: String ], TuiViewData::Text);

make_view!(pub ptext, PTextConstrArgs {
    color: Option<Color>
} [ text: String ], TuiViewData::Text { wrap_mode: TextWrapMode::Word });

make_view!(pub color, [ color: Color ], TuiViewData::Color);

make_view!(pub border, BorderConstrArgs {
    color: Option<Color>
} [ style: BorderStyle ], TuiViewData::Border);

make_view!(pub hdivider, HDividerConstrArgs {
    color: Option<Color>
} [ style: DividerStyle ], TuiViewData::Divider { direction: DividerDirection::Horizontal });

make_view!(pub vdivider, VDividerConstrArgs {
    color: Option<Color>
} [ style: DividerStyle ], TuiViewData::Divider { direction: DividerDirection::Vertical });

#[cfg(feature = "images")]
make_view!(pub source, SourceConstrArgs {
    handle_aspect_ratio: HandleAspectRatio
} [ source: Source ], TuiViewData::Source);