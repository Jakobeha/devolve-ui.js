pub macro mt {
    (0) => {
        $crate::core::view::layout::bounds::Measurement::Zero
    },
    (prev) => {
        $crate::core::view::layout::bounds::Measurement::Prev
    },
    ($lit:literal) => {
        $crate::core::view::layout::bounds::Measurement::Units($lit as f32)
    },
    ($lit:literal px) => {
        $crate::core::view::layout::bounds::Measurement::Pixels($lit as f32)
    },
    ($lit:literal %) => {
        $crate::core::view::layout::bounds::Measurement::Fraction($lit as f32 / 100f32)
    },
    ($store:ident = $expr:tt) => {
        $crate::core::view::layout::bounds::Measurement::Store(stringify!($store), Box::new(mt!($expr)))
    },
    ($lhs:tt * $rhs:literal) => {
        $crate::core::view::layout::bounds::Measurement::Mul(Box::new(mt!($lhs)), $rhs as f32)
    },
    ($lhs:tt / $rhs:literal) => {
        $crate::core::view::layout::bounds::Measurement::Div(Box::new(mt!($lhs)), $rhs as f32)
    },
    ($lhs:tt + $rhs:tt) => {
        $crate::core::view::layout::bounds::Measurement::Add(Box::new(mt!($lhs)), Box::new(mt!($rhs)))
    },
    ($lhs:tt - $rhs:tt) => {
        $crate::core::view::layout::bounds::Measurement::Sub(Box::new(mt!($lhs)), Box::new(mt!($rhs)))
    },
    ($lhs:tt $lhs2:tt * $rhs:literal) => {
        $crate::core::view::layout::bounds::Measurement::Mul(Box::new(mt!($lhs $lhs2)), $rhs as f32)
    },
    ($lhs:tt $lhs2:tt / $rhs:literal) => {
        $crate::core::view::layout::bounds::Measurement::Div(Box::new(mt!($lhs $lhs2)), $rhs as f32)
    },
    ($lhs:tt $lhs2:tt + $rhs:tt) => {
        $crate::core::view::layout::bounds::Measurement::Add(Box::new(mt!($lhs $lhs2)), Box::new(mt!($rhs)))
    },
    ($lhs:tt $lhs2:tt - $rhs:tt) => {
        $crate::core::view::layout::bounds::Measurement::Sub(Box::new(mt!($lhs $lhs2)), Box::new(mt!($rhs)))
    },
    ($lhs:tt + $rhs:tt $rhs2:tt) => {
        $crate::core::view::layout::bounds::Measurement::Add(Box::new(mt!($lhs)), Box::new(mt!($rhs $rhs2)))
    },
    ($lhs:tt - $rhs:tt $rhs2:tt) => {
        $crate::core::view::layout::bounds::Measurement::Sub(Box::new(mt!($lhs)), Box::new(mt!($rhs $rhs2)))
    },
    ($lhs:tt $lhs2:tt + $rhs:tt $rhs2:tt) => {
        $crate::core::view::layout::bounds::Measurement::Add(Box::new(mt!($lhs $lhs2)), Box::new(mt!($rhs $rhs2)))
    },
    ($lhs:tt $lhs2:tt - $rhs:tt $rhs2:tt) => {
        $crate::core::view::layout::bounds::Measurement::Sub(Box::new(mt!($lhs $lhs2)), Box::new(mt!($rhs $rhs2)))
    },
    ($load:ident) => {
        $crate::core::view::layout::bounds::Measurement::Load(strifify!($load))
    },
    (($($expr:tt)+)) => {
        mt!($($expr)+)
    },
}

pub macro smt {
    (auto) => { None },
    ($($expr:tt)*) => { Some(mt!($($expr)*)) }
}