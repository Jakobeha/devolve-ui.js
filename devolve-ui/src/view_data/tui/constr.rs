#![allow(non_upper_case_globals)]

use crate::core::component::node::VNode;
use crate::core::view::color::Color;
use crate::core::view::constr::{constr_view, VViewConstrArgs};
use crate::core::view::layout::bounds::Measurement;
use crate::core::view::layout::parent_bounds::{LayoutDirection, SubLayout};
use crate::view_data::attrs::TextWrapMode;
use crate::view_data::tui::tui::TuiViewData;

pub fn hbox(view_args: VViewConstrArgs, gap: Measurement, children: Vec<VNode<TuiViewData>>) -> VNode<TuiViewData> {
    constr_view(view_args, TuiViewData::Box {
        children,
        sub_layout: SubLayout {
            direction: LayoutDirection::Horizontal,
            gap
        },
        clip: false,
        extend: false
    })
}

pub macro hbox {
    ({ $field:ident : $value:expr }, $gap:expr, $children:expr)  => {
        hbox(VViewConstrArgs {
            $field: $value,
            ..VViewConstrArgs::default()
        }, $gap, $children)
    },
    ({ $field:ident : $value:expr }, $children:expr)  => {
        hbox(VViewConstrArgs {
            $field: $value,
            ..VViewConstrArgs::default()
        }, Measruement::Zero, $children)
    }
}

pub fn vbox(view_args: VViewConstrArgs, gap: Measurement, children: Vec<VNode<TuiViewData>>) -> VNode<TuiViewData> {
    constr_view(view_args, TuiViewData::Box {
        children,
        sub_layout: SubLayout {
            direction: LayoutDirection::Vertical,
            gap
        },
        clip: false,
        extend: false
    })
}

pub macro vbox {
    ({ $field:ident : $value:expr }, $gap:expr, $children:expr)  => {
        vbox(VViewConstrArgs {
            $field: $value,
            ..VViewConstrArgs::default()
        }, $gap, $children)
    },
    ({ $field:ident : $value:expr }, $children:expr)  => {
        vbox(VViewConstrArgs {
            $field: $value,
            ..VViewConstrArgs::default()
        }, Measruement::Zero, $children)
    }
}

pub fn zbox(view_args: VViewConstrArgs, children: Vec<VNode<TuiViewData>>) -> VNode<TuiViewData> {
    constr_view(view_args, TuiViewData::Box {
        children,
        sub_layout: SubLayout {
            direction: LayoutDirection::Overlap,
            gap: Measurement::Zero
        },
        clip: false,
        extend: false
    })
}

pub macro zbox({ $field:ident : $value:expr }, $children:expr) {
    zbox(VViewConstrArgs {
        $field: $value,
        ..VViewConstrArgs::default()
    }, $children)
}

pub fn clip_zbox(view_args: VViewConstrArgs, children: Vec<VNode<TuiViewData>>) -> VNode<TuiViewData> {
    constr_view(view_args, TuiViewData::Box {
        children,
        sub_layout: SubLayout {
            direction: LayoutDirection::Overlap,
            gap: Measurement::Zero
        },
        clip: true,
        extend: false
    })
}

pub macro clip_zbox({ $field:ident : $value:expr }, $children:expr) {
    clip_zbox(VViewConstrArgs {
        $field: $value,
        ..VViewConstrArgs::default()
    }, $children)
}

pub fn extend_zbox(view_args: VViewConstrArgs, children: Vec<VNode<TuiViewData>>) -> VNode<TuiViewData> {
    constr_view(view_args, TuiViewData::Box {
        children,
        sub_layout: SubLayout {
            direction: LayoutDirection::Overlap,
            gap: Measurement::Zero
        },
        clip: false,
        extend: true
    })
}


pub macro extend_zbox({ $field:ident : $value:expr }, $children:expr) {
    extend_zbox(VViewConstrArgs {
        $field: $value,
        ..VViewConstrArgs::default()
    }, $children)
}

pub fn ce_zbox(view_args: VViewConstrArgs, children: Vec<VNode<TuiViewData>>) -> VNode<TuiViewData> {
    constr_view(view_args, TuiViewData::Box {
        children,
        sub_layout: SubLayout {
            direction: LayoutDirection::Overlap,
            gap: Measurement::Zero
        },
        clip: true,
        extend: true
    })
}


pub macro ce_zbox({ $field:ident : $value:expr }, $children:expr) {
    ce_zbox(VViewConstrArgs {
        $field: $value,
        ..VViewConstrArgs::default()
    }, $children)
}

pub fn stext(view_args: VViewConstrArgs, color: Option<Color>, text: String) -> VNode<TuiViewData> {
    constr_view(view_args, TuiViewData::Text {
        text,
        color,
        wrap_mode: TextWrapMode::Undefined
    })
}

pub macro stext {
    ({ $field:ident : $value:expr }, $color:expr, $text:expr) => {
        stext(VViewConstrArgs {
            $field: $value,
            ..VViewConstrArgs::default()
        }, $color, $text)
    },
    ({ $field:ident : $value:expr }, $text:expr) => {
        stext(VViewConstrArgs {
            $field: $value,
            ..VViewConstrArgs::default()
        }, None, $text)
    }
}

pub fn ptext(view_args: VViewConstrArgs, color: Option<Color>, text: String) -> VNode<TuiViewData> {
    constr_view(view_args, TuiViewData::Text {
        text,
        color,
        wrap_mode: TextWrapMode::Word
    })
}

pub macro ptext {
    ({ $field:ident : $value:expr }, $color:expr, $text:expr) => {
        ptext(VViewConstrArgs {
            $field: $value,
            ..VViewConstrArgs::default()
        }, $color, $text)
    },
    ({ $field:ident : $value:expr }, $text:expr) => {
        ptext(VViewConstrArgs {
            $field: $value,
            ..VViewConstrArgs::default()
        }, None, $text)
    }
}