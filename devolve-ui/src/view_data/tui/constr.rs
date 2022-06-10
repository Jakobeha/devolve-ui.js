//! Terse constructors for `TuiViewData` views.

#![allow(non_upper_case_globals)]

use crate::core::component::node::VNode;
use crate::core::view::color::Color;
#[allow(unused_imports)] // Needed for IntelliJ macro expansion
use crate::core::view::constr::{_make_view, constr_view, make_view, VViewConstrArgs};
use crate::core::view::layout::bounds::Measurement;
use crate::core::view::layout::parent_bounds::{LayoutDirection, SubLayout};
use crate::view_data::attrs::{BorderStyle, DividerDirection, DividerStyle, TextWrapMode};
use crate::view_data::tui::terminal_image::{HandleAspectRatio, Source};
use crate::view_data::tui::tui::TuiViewData;

#[derive(Default)]
pub struct BoxConstrArgs {
    pub gap: Measurement,
    pub children: Vec<VNode<TuiViewData>>,
    pub clip: bool,
    pub extend: bool
}

macro _box2(($d:tt) @ $name:ident, $layout_direction: expr) {
    pub fn $name(view_args: VViewConstrArgs, data_args: BoxConstrArgs) -> VNode<TuiViewData> {
        constr_view(view_args, TuiViewData::Box {
            children: data_args.children,
            sub_layout: SubLayout {
                direction: $layout_direction,
                gap: data_args.gap
            },
            clip: data_args.clip,
            extend: data_args.extend
        })
    }

    pub macro $name({ $d( $d view_field:ident : $d view_value:expr ),* }, { $d( $d data_field:ident: $d data_value:expr ),* } $d( , $d children:expr )?) {
        $name(VViewConstrArgs {
            $d( $d view_field : $d view_value, )*
            ..VViewConstrArgs::default()
        }, BoxConstrArgs {
            $d( $d data_field : $d data_value, )*
            $d( children: $d children, )?
            ..BoxConstrArgs::default()
        })
    }
}

macro _box($name:ident, $layout_direction:expr) {
    _box2!(($) @ $name, $layout_direction);
}

_box!(hbox, LayoutDirection::Horizontal);
_box!(vbox, LayoutDirection::Vertical);
_box!(zbox, LayoutDirection::Overlap);

pub fn ce_zbox(view_args: VViewConstrArgs, children: Vec<VNode<TuiViewData>>) -> VNode<TuiViewData> {
    zbox(view_args, BoxConstrArgs {
        children,
        clip: true,
        extend: true,
        ..BoxConstrArgs::default()
    })
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

make_view!(pub source, SourceConstrArgs {
    handle_aspect_ratio: HandleAspectRatio
} [ source: Source ], TuiViewData::Source);