use crate::core::view::macros::make_view;

make_view!(pub macro hbox, TuiViewData::Box, {
    children: vec![],
    sub_layout: {
        direction: LayoutDirection::Horizontal
        ..Default::default()
    },
    clip: false,
    extend: false
});

make_view!(pub macro vbox, TuiViewData::Box, {
    children: vec![],
    sub_layout: {
        direction: LayoutDirection::Vertical
        ..Default::default()
    },
    clip: false,
    extend: false
});

make_view!(pub macro zbox, TuiViewData::Box, {
    children: vec![],
    sub_layout: {
        direction: LayoutDirection::Overlap
        ..Default::default()
    },
    clip: false,
    extend: false
});

make_view!(pub macro clip_box, TuiViewData::Box, {
    children: vec![],
    sub_layout: Default::default(),
    clip: true,
    extend: false
});

make_view!(pub macro ce_box, TuiViewData::Box, {
    children: vec![],
    sub_layout: Default::default(),
    clip: true,
    extend: true
});

make_view!(pub macro text, TuiViewData::Text, {
    text: String::new(),
    color: Color::Black,
    wrap: TextWrapMode::Undefined
});

make_view!(pub macro ptext, TuiViewData::Text, {
    text: String::new(),
    color: Color::Black,
    wrap: TextWrapMode::Word
});