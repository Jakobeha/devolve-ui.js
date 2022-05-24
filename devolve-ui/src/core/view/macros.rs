use crate::core::component::node::VNode;
use crate::core::view::layout::bounds::Bounds;
use crate::core::view::view::VView;

macro mt {
    (0) => {
        $crate::Measurement::Zero
    },
    (prev) => {
        $crate::Measurement::Prev
    },
    ($lit:literal) => {
        $crate::Measurement::Units($lit as f32)
    },
    ($lit:literal px) => {
        $crate::Measurement::Pixels($lit as f32)
    },
    ($lit:literal %) => {
        $crate::Measurement::Fraction($lit as f32 / 100f32)
    },
    ($store:ident = $expr:tt) => {
        $crate::Measurement::Store(stringify!($store), $crate::Box::new(mt!($expr)))
    },
    ($lhs:tt * $rhs:literal) => {
        $crate::Measurement::Mul($crate::Box::new(mt!($lhs)), $rhs as f32)
    },
    ($lhs:tt / $rhs:literal) => {
        $crate::Measurement::Div($crate::Box::new(mt!($lhs)), $rhs as f32)
    },
    ($lhs:tt + $rhs:tt) => {
        $crate::Measurement::Add($crate::Box::new(mt!($lhs)), $crate::Box::new(mt!($rhs)))
    },
    ($lhs:tt - $rhs:tt) => {
        $crate::Measurement::Sub($crate::Box::new(mt!($lhs)), $crate::Box::new(mt!($rhs)))
    },
    ($load:ident) => {
        $crate::Measurement::Load(strifify!($load))
    },
    (($expr:tt)) => {
        mt!($expr)
    }
}

// Usage: `view!(TuiViewData::Text | size: (80, 40), anchor: (0.5, 0.5), is_visible: is_visible, text: "Hello world!" })`
macro view(
    $constr:path
    |
    $( key: $key:ident , )?
    $( layout : $layout_type:expr , )?
    $( pos : ($x:tt , $y:tt $(, $z: tt)? ) , )?
    $( size : ($width:tt , $height:tt) , )?
    $( anchor : ($anchor_x:expr , $anchor_y:expr) , )?
    // $( gap : $gap:tt , )?
    $( is_visible : $visible:expr , )?
    |
    $( $attr:ident : $value:expr , )*
    $text:literal
    $( [ $( $child:expr ),* ] )?
) {
    $crate::VNode::View($crate::Box::new($crate::VView::new(
        $crate::Bounds {
            $(layout_type: $layout_type)?,
            $(x: mt!($x))?,
            $(y: mt!($y))?,
            $(z: $z)?,
            $(width: mt!($width))?,
            $(height: mt!($height))?,
            $(anchor_x: $anchor_x as f32)?,
            $(anchor_y: $anchor_y as f32)?,
            ..$crate::Bounds::default()
        },
        true $( && $is_visible )?,
        None $( .or(Some($key)) )?,
        $constr {
            $( $attr : $value , )*
            $( text : $text , )?
            $( children: vec![$( $child ),*] )?
        }
    )))
}

#[cfg(test)]
mod tests {
    use crate::core::view::layout::bounds::{LayoutPosition, Measurement};
    use crate::core::view::macros::{view, mt};
    use crate::view_data::attrs::TextWrapMode;
    use crate::view_data::tui::TuiViewData;

    #[test]
    fn test_tt() {
        assert_eq!(mt!(50%), Measurement::Fraction(0.5f32));
        assert_eq!(mt!(prev * 2), Measurement::Mul(Box::new(Measurement::Prev), 2f32));
        assert_eq!(mt!(test = ((prev + 10) * 2)), Measurement::Store("test", Box::new(Measurement::Mul(Box::new(Measurement::Add(Box::new(Measurement::Prev), Box::new(Measurement::Units(10f32)))), 2f32))));
    }

    #[test]
    fn test_view() {
        view!(TuiViewData::Box | layout: LayoutPosition::Absolute, pos: (0, 0, 1), size: (100%, 100%), []);
    }
}