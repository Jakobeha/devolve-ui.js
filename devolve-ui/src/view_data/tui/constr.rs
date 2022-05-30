#![allow(non_upper_case_globals)]

use crate::core::component::node::VNode;
use crate::core::view::color::Color;
use crate::core::view::constr::{constr_view, VViewConstrArgs};
use crate::core::view::layout::bounds::Measurement;
use crate::core::view::layout::parent_bounds::{LayoutDirection, SubLayout};
use crate::view_data::attrs::TextWrapMode;
use crate::view_data::tui::tui::TuiViewData;

#[derive(Default)]
pub struct BoxConstrArgs {
    pub gap: Measurement,
    pub children: Vec<VNode<TuiViewData>>,
    pub clip: bool,
    pub extend: bool
}

#[derive(Default)]
pub struct TextConstrArgs {
    pub color: Option<Color>,
    pub wrap: TextWrapMode,
    pub text: String
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

macro _box($name:ident, $layout_direction: expr) {
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


pub fn text(view_args: VViewConstrArgs, data_args: TextConstrArgs) -> VNode<TuiViewData> {
    constr_view(view_args, TuiViewData::Text {
        text: data_args.text,
        color: data_args.color,
        wrap_mode: data_args.wrap
    })
}

pub macro text({ $( $view_field:ident: $view_value:expr ),* }, { $( $data_field:ident: $data_value:expr ),* } $( , $text:expr )?) {
    text(VViewConstrArgs {
        $( $view_field : $view_value, )*
        ..VViewConstrArgs::default()
    }, TextConstrArgs {
        $( $data_field : $data_value, )*
        $( text: $text, )?
        ..TextConstrArgs::default()
    })
}

pub fn ptext(view_args: VViewConstrArgs, mut data_args: TextConstrArgs) -> VNode<TuiViewData> {
    data_args.wrap = TextWrapMode::Word;
    text(view_args, data_args)
}

pub macro ptext({ $( $view_field:ident: $view_value:expr ),* }, { $( $data_field:ident: $data_value:expr ),* } $( , $text:expr )?) {
    ptext(VViewConstrArgs {
        $( $view_field : $view_value, )*
        ..VViewConstrArgs::default()
    }, TextConstrArgs {
        $( $data_field : $data_value, )*
        $( text: $text, )?
        ..TextConstrArgs::default()
    })
}

