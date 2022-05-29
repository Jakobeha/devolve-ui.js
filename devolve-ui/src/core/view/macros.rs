use std::borrow::Cow;
use crate::core::view::layout::bounds::{Bounds, LayoutPosition1D, Measurement};

#[derive(Default)]
#[allow(dead_code)]
pub struct ViewMacroBuiltinAttrs {
    bounds: Option<Bounds>,
    layout: Option<LayoutPosition1D>,
    layout_x: Option<LayoutPosition1D>,
    layout_y: Option<LayoutPosition1D>,
    pos: Option<(Measurement, Measurement)>,
    x: Option<Measurement>,
    y: Option<Measurement>,
    z: Option<i32>,
    size: Option<(Option<Measurement>, Option<Measurement>)>,
    width: Option<Option<Measurement>>,
    height: Option<Option<Measurement>>,
    anchor: Option<(f32, f32)>,
    anchor_x: Option<f32>,
    anchor_y: Option<f32>,
    is_visible: Option<bool>,
    key: Option<Cow<'static, str>>,
}


/// Usage: `make_view!(text!, TuiViewData::Text, { color: Color::BLACK })`
pub macro make_view(
    $vis:vis macro $name:ident,
    $constr:ident :: $constr2:ident,
    $( # { $( $builtin_attr:ident : $builtin_value:expr ),* $( , )? } )?
    $( ( $( $tuple_value:expr ),* $( , )? ) )?
    $( { $( $field:ident : $field_value:expr ),* $( , )? } )?
) {
    /// Usage: `$name!(TuiViewData::Text #{ size: (smt!(80), smt!(40)), anchor: (0.5, 0.5), is_visible: is_visible } { color: Color::BLUE } "Hello world!")`
    $vis macro $name(
        $$( # { $$( $$builtin_attr:ident : $$builtin_value:expr ),* $$( , )? } )?
        $$( ( $$( $$tuple_value:expr ),* $$( , )? ) )?
        $$( { $$( $$field:ident : $$field_value:expr ),* $$( , )? } )?
        $$( $$text:literal )?
        $$( [ $$( $$child:expr ),* $$( , )? ] )?
    ) {
        // The way we assign attrs is to first set to defaults,
        // then macro-expand using (value.$$field = $$value).
        // We do this for both builtins and custom attrs.

        // Assign builtin attrs
        // Trick to get around hygeine
        let $crate::core::view::macros::ViewMacroBuiltinAttrs {
            bounds,
            layout,
            layout_x,
            layout_y,
            pos,
            x,
            y,
            z,
            size,
            width,
            height,
            anchor,
            anchor_x,
            anchor_y,
            is_visible,
            key
        } = ::dedup_struct_fields::dedup_struct_fields!($crate::core::view::macros::ViewMacroBuiltinAttrs {
            $$( $$( $$builtin_attr : Some($$builtin_value), )* )?
            $( $( $builtin_attr : Some($builtin_value), )* )?
            ..$crate::core::view::macros::ViewMacroBuiltinAttrs::default()
        });

        let bounds = bounds.unwrap_or($crate::core::view::layout::bounds::Bounds::default());
        let pos = pos.unwrap_or((bounds.x, bounds.y));
        let size = size.unwrap_or((bounds.width, bounds.height));
        let anchor = anchor.unwrap_or((bounds.anchor_x, bounds.anchor_y));
        let bounds = $crate::core::view::layout::bounds::Bounds {
            layout: $crate::core::view::layout::bounds::LayoutPosition {
                x: layout_x.or(layout).unwrap_or(bounds.layout.x),
                y: layout_y.or(layout).unwrap_or(bounds.layout.y)
            },
            x: x.unwrap_or(pos.0),
            y: y.unwrap_or(pos.1),
            z: z.unwrap_or(bounds.z),
            width: width.unwrap_or(size.0),
            height: height.unwrap_or(size.1),
            anchor_x: anchor_x.unwrap_or(anchor.0),
            anchor_y: anchor_y.unwrap_or(anchor.1)
        };
        let is_visible = is_visible.unwrap_or(true);

        // Assign custom attrs
        let mut d = $constr :: $constr2 $( (
            $( $tuple_value ),*
        ) )? $( {
            $( $field : $field_value ),*
        } )?;
        // TODO overrides for tuple attrs
        $(
            if let $constr :: $constr2 {
                $( mut $field ),*
            } = d {
                d = ::dedup_struct_fields::dedup_struct_fields!($constr :: $constr2 {
                    $$( $$( $$field: $$field_value, )* )?
                    $$( text: $$text, )?
                    $$( children: vec![$$( $$child ),*], )?
                    $( $field ),*
                });
            } else {
                panic!("impossible")
            }
        )?

        $crate::core::component::node::VNode::View(Box::new($crate::core::view::view::VView::new(
            bounds,
            is_visible,
            key,
            d
        )))
    }
}

/* /// Usage: `remake_view!(red_text, text, { color: Color::RED })`
pub macro remake_view(
    $name:ident !,
    $original:ident !,
    $( # { $( $builtin_attr:ident : $builtin_value:expr ),* $( , )? } )?
    $( ( $( $tuple_value:expr ),* $( , )? ) )?
    $( { $( $field:ident : $field_value:expr ),* $( , )? } )?
    $( $text:literal )?
    $( [ $( $child:expr ),* $( , )? ] )?
) {
    /// Usage: `$name!(TuiViewData::Text(size: (80, 40), anchor: (0.5, 0.5), is_visible: is_visible) { color: Color::BLUE } "Hello world!")`
    pub macro $name(
        $$( # { $$( $$builtin_attr:ident : $$builtin_value:expr ),* } )?
        $$( ( $$( $$tuple_value:expr ),* ) )?
        $$( { $$( $$field:ident : $$field_value:expr ),* } )?
        $$( $$text:literal )?
        $$( [ $$( $$child:expr ),* ] )?
    ) {
        $original!(
            # {
                $$( $$( $$builtin_attr : $$builtin_value, )* )?
                $( $( $builtin_attr : $builtin_value, )* )?
            }
            $$( ( $$( $$tuple_value, )* $( $( $tuple_value, )* )? ) )?
            $$( { $$( $$field : $$field_value, )* $( $( $field_value, )* )? } )?
            $$( $$text )? $( $text )?
            $$( [ $$( $$child, )* $( $( $child, )* )? ] )?
        )
    }
} */

#[cfg(test)]
#[cfg(feature = "tui")]
mod tests {
    use crate::core::view::layout::bounds::{LayoutPosition1D, Measurement};
    use crate::core::view::macros::{make_view, remake_view, mt, smt};
    use crate::view_data::tui::tui::TuiViewData;

    #[test]
    fn test_tt() {
        assert_eq!(mt!(50%), Measurement::Fraction(0.5f32));
        assert_eq!(mt!(prev * 2), Measurement::Mul(Box::new(Measurement::Prev), 2f32));
        assert_eq!(mt!(test = (prev + 10)), Measurement::Store("test", Box::new(Measurement::Add(Box::new(Measurement::Prev), Box::new(Measurement::Units(10f32))))));
        assert_eq!(mt!(test = ((prev + 10) * 2)), Measurement::Store("test", Box::new(Measurement::Mul(Box::new(Measurement::Add(Box::new(Measurement::Prev), Box::new(Measurement::Units(10f32)))), 2f32))));
    }

    make_view!(macro hbox, TuiViewData::Box, {
        children: vec![],
        sub_layout: {
            direction: LayoutDirection::Horizontal
            ..Default::default()
        },
        clip: false,
        extend: false
    });

    make_view!(macro vbox, TuiViewData::Box, {
        children: vec![],
        sub_layout: {
            direction: LayoutDirection::Vertical
            ..Default::default()
        },
        clip: false,
        extend: false
    });

    make_view!(macro zbox, TuiViewData::Box, {
        children: vec![],
        sub_layout: {
            direction: LayoutDirection::Overlap
            ..Default::default()
        },
        clip: false,
        extend: false
    });

    make_view!(macro clip, TuiViewData::Box, {
        children: vec![],
        sub_layout: Default::default(),
        clip: true,
        extend: false
    });

    make_view!(macro fix_size, TuiViewData::Box, {
        children: vec![],
        sub_layout: Default::default(),
        clip: true,
        extend: true
    });

    #[test]
    fn test_view() {
        box_!(#{
            layout: LayoutPosition1D::Relative,
            pos: (mt!(0), mt!(0)),
            z: 1,
            size: (smt!(100%), smt!(100%))
        } []);
    }
}