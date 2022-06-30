pub macro _mt {
    ($mt:ident) => {
        $mt
    },
    ($mt:ident $sign:tt $lit:literal u $($rest:tt)*) => {{
        $mt = ($mt $sign $crate::core::view::layout::measurement::Measurement1 {
            value: $crate::core::view::layout::measurement::MeasurementValue::new(
                $lit as f32,
                $crate::core::view::layout::measurement::MeasurementDebugSymbol::Literal
            ),
            unit: $crate::core::view::layout::measurement::MeasurementUnit::Units,
        }).expect("measurement macro is too large");
        _mt!($mt $($rest)*)
    }},
    ($mt:ident $sign:tt $lit:literal px $($rest:tt)*) => {{
        $mt = ($mt $sign $crate::core::view::layout::measurement::Measurement1 {
            value: $crate::core::view::layout::measurement::MeasurementValue::new(
                $lit as f32,
                $crate::core::view::layout::measurement::MeasurementDebugSymbol::Literal
            ),
            unit: $crate::core::view::layout::measurement::MeasurementUnit::Pixels,
        }).expect("measurement macro is too large");
        _mt!($mt $($rest)*)
    }},
    ($mt:ident $sign:tt $lit:literal % $($rest:tt)*) => {{
        $mt = ($mt $sign $crate::core::view::layout::measurement::Measurement1 {
            value: $crate::core::view::layout::measurement::MeasurementValue::new(
                $lit as f32,
                $crate::core::view::layout::measurement::MeasurementDebugSymbol::Literal
            ),
            unit: $crate::core::view::layout::measurement::MeasurementUnit::PercentOfParent,
        }).expect("measurement macro is too large");
        _mt!($mt $($rest)*)
    }},
    ($mt:ident $sign:tt $lit:literal * prev $($rest:tt)*) => {{
        $mt = ($mt $sign $crate::core::view::layout::measurement::Measurement1 {
            value: $crate::core::view::layout::measurement::MeasurementValue::new(
                $lit as f32,
                $crate::core::view::layout::measurement::MeasurementDebugSymbol::Literal
            ),
            unit: $crate::core::view::layout::measurement::MeasurementUnit::OfPrev,
        }).expect("measurement macro is too large");
        _mt!($mt $($rest)*)
    }},
    ($mt:ident $sign:tt $lit:literal * load($id:ident) $($rest:tt)*) => {{
        $mt = ($mt $sign $crate::core::view::layout::measurement::Measurement1 {
            value: $crate::core::view::layout::measurement::MeasurementValue::new(
                $lit as f32,
                $crate::core::view::layout::measurement::MeasurementDebugSymbol::Literal
            ),
            unit: $crate::core::view::layout::measurement::MeasurementUnit::OfLoad($crate::core::misc::ident::Ident::try_from(stringify!($id)).expect("load ident is too large")),
        }).expect("measurement macro is too large");
        _mt!($mt $($rest)*)
    }},
    ($mt:ident $sign:tt prev / $lit:literal $($rest:tt)*) => {{
        $mt = ($mt $sign $crate::core::view::layout::measurement::Measurement1 {
            value: $crate::core::view::layout::measurement::MeasurementValue::new(
                1f32 / ($lit as f32),
                $crate::core::view::layout::measurement::MeasurementDebugSymbol::Literal
            ),
            unit: $crate::core::view::layout::measurement::MeasurementUnit::OfPrev,
        }).expect("measurement macro is too large");
        _mt!($mt $($rest)*)
    }},
    ($mt:ident $sign:tt load($id:ident) / $lit:literal $($rest:tt)*) => {{
        $mt = ($mt $sign $crate::core::view::layout::measurement::Measurement1 {
            value: $crate::core::view::layout::measurement::MeasurementValue::new(
                1f32 / ($lit as f32),
                $crate::core::view::layout::measurement::MeasurementDebugSymbol::Literal
            ),
            unit: $crate::core::view::layout::measurement::MeasurementUnit::OfLoad($crate::core::misc::ident::Ident::try_from(stringify!($id)).expect("load ident is too large")),
        }).expect("measurement macro is too large");
        _mt!($mt $($rest)*)
    }},
    ($mt:ident $sign:tt {$exp:expr} u $($rest:tt)*) => {{
        $mt = ($mt $sign $crate::core::view::layout::measurement::Measurement1 {
            value: $crate::core::view::layout::measurement::MeasurementValue::new(
                $exp as f32,
                $crate::core::view::layout::measurement::MeasurementDebugSymbol::Expr(stringify!($exp))
            ),
            unit: $crate::core::view::layout::measurement::MeasurementUnit::Units,
        }).expect("measurement macro is too large");
        _mt!($mt $($rest)*)
    }},
    ($mt:ident $sign:tt {$exp:expr} px $($rest:tt)*) => {{
        $mt = ($mt $sign $crate::core::view::layout::measurement::Measurement1 {
            value: $crate::core::view::layout::measurement::MeasurementValue::new(
                $exp as f32,
                $crate::core::view::layout::measurement::MeasurementDebugSymbol::Expr(stringify!($exp))
            ),
            unit: $crate::core::view::layout::measurement::MeasurementUnit::Pixels,
        }).expect("measurement macro is too large");
        _mt!($mt $($rest)*)
    }},
    ($mt:ident $sign:tt {$exp:expr} % $($rest:tt)*) => {{
        $mt = ($mt $sign $crate::core::view::layout::measurement::Measurement1 {
            value: $crate::core::view::layout::measurement::MeasurementValue::new(
                $exp as f32,
                $crate::core::view::layout::measurement::MeasurementDebugSymbol::Expr(stringify!($exp))
            ),
            unit: $crate::core::view::layout::measurement::MeasurementUnit::PercentOfParent,
        }).expect("measurement macro is too large");
        _mt!($mt $($rest)*)
    }},
    ($mt:ident $sign:tt {$exp:expr} * prev $($rest:tt)*) => {{
        $mt = ($mt $sign $crate::core::view::layout::measurement::Measurement1 {
            value: $crate::core::view::layout::measurement::MeasurementValue::new(
                $exp as f32,
                $crate::core::view::layout::measurement::MeasurementDebugSymbol::Expr(stringify!($exp))
            ),
            unit: $crate::core::view::layout::measurement::MeasurementUnit::OfPrev,
        }).expect("measurement macro is too large");
        _mt!($mt $($rest)*)
    }},
    ($mt:ident $sign:tt {$exp:expr} * load($id:ident) $($rest:tt)*) => {{
        $mt = ($mt $sign $crate::core::view::layout::measurement::Measurement1 {
            value: $crate::core::view::layout::measurement::MeasurementValue::new(
                $exp as f32,
                $crate::core::view::layout::measurement::MeasurementDebugSymbol::Expr(stringify!($exp))
            ),
            unit: $crate::core::view::layout::measurement::MeasurementUnit::OfLoad($crate::core::misc::ident::Ident::try_from(stringify!($id)).expect("load ident is too large")),
        }).expect("measurement macro is too large");
        _mt!($mt $($rest)*)
    }},
    ($mt:ident $sign:tt prev / {$exp:expr} $($rest:tt)*) => {{
        $mt = ($mt $sign $crate::core::view::layout::measurement::Measurement1 {
            value: $crate::core::view::layout::measurement::MeasurementValue::new(
                1f32 / ($exp as f32),
                $crate::core::view::layout::measurement::MeasurementDebugSymbol::Expr(stringify!(/ $exp))
            ),
            unit: $crate::core::view::layout::measurement::MeasurementUnit::OfPrev,
        }).expect("measurement macro is too large");
        _mt!($mt $($rest)*)
    }},
    ($mt:ident $sign:tt load($id:ident) / {$exp:expr} $($rest:tt)*) => {{
        $mt = ($mt $sign $crate::core::view::layout::measurement::Measurement1 {
            value: $crate::core::view::layout::measurement::MeasurementValue::new(
                1f32 / ($exp as f32),
                $crate::core::view::layout::measurement::MeasurementDebugSymbol::Expr(stringify!(/ $exp))
            ),
            unit: $crate::core::view::layout::measurement::MeasurementUnit::OfLoad($crate::core::misc::ident::Ident::try_from(stringify!($id)).expect("load ident is too large")),
        }).expect("measurement macro is too large");
        _mt!($mt $($rest)*)
    }},
    ($mt:ident $sign:tt prev $($rest:tt)*) => {{
        $mt = ($mt $sign $crate::core::view::layout::measurement::Measurement1 {
            value: $crate::core::view::layout::measurement::MeasurementValue::new(
                1f32,
                $crate::core::view::layout::measurement::MeasurementDebugSymbol::Literal
            ),
            unit: $crate::core::view::layout::measurement::MeasurementUnit::OfPrev,
        }).expect("measurement macro is too large");
        _mt!($mt $($rest)*)
    }},
    ($mt:ident $sign:tt load($id:ident) $($rest:tt)*) => {{
        $mt = ($mt $sign $crate::core::view::layout::measurement::Measurement1 {
            value: $crate::core::view::layout::measurement::MeasurementValue::new(
                1f32,
                $crate::core::view::layout::measurement::MeasurementDebugSymbol::Literal
            ),
            unit: $crate::core::view::layout::measurement::MeasurementUnit::OfLoad($crate::core::misc::ident::Ident::try_from(stringify!($id)).expect("load ident is too large")),
        }).expect("measurement macro is too large");
        _mt!($mt $($rest)*)
    }}
}

pub macro mt {
    ($store:ident = $($rest:tt)*) => {
        {
            let mut mt = $crate::core::view::layout::measurement::Measurement::ZERO;
            mt.store = Some($crate::core::misc::ident::Ident::try_from(stringify!($store)).expect("store ident is too large"));
            _mt!(mt + $($rest)*)
        }
    },
    ($($rest:tt)*) => {
        {
            let mut mt = $crate::core::view::layout::measurement::Measurement::ZERO;
            _mt!(mt + $($rest)*)
        }
    }
}

pub macro smt {
    (auto) => { $crate::core::view::layout::measurement::SizeMeasurement::AUTO },
    ($($rest:tt)*) => { $crate::core::view::layout::measurement::SizeMeasurement::from(mt!($($rest)*)) }
}

#[cfg(test)]
mod test {
    #[allow(unused_imports)] // Would be needed by IntelliJ, it's not actually needed because unforunately IntelliJ resolution still doesn't work
    use crate::core::view::layout::macros::{_mt, mt, smt};

    #[test]
    fn test_measurements() {
        let foo = 1;
        let bar = 5;
        let baz = 12;
        assert_eq!(mt!(1 u).to_string(), "1");
        assert_eq!(mt!(1 px).to_string(), "1px");
        assert_eq!(mt!(1 %).to_string(), "1%");
        assert_eq!(mt!(prev).to_string(), "prev");
        assert_eq!(mt!(2 * prev).to_string(), "2*prev");
        assert_eq!(mt!(prev / 2).to_string(), "0.5*prev");
        assert_eq!(mt!(load(ident)).to_string(), "load(ident)");
        assert_eq!(mt!(2 * load(ident)).to_string(), "2*load(ident)");
        assert_eq!(mt!(1 px + 5 %).to_string(), "(1px + 5%)");
        assert_eq!(mt!(1 px - 5 %).to_string(), "(1px - 5%)");
        assert_eq!(mt!(1 px + 5 u + 12 %).to_string(), "(1px + 5 + 12%)");
        assert_eq!(mt!(1 px + 5 u - 12 %).to_string(), "(1px + 5 - 12%)");
        assert_eq!(mt!({foo} px + {bar} u - {baz} %).to_string(), "(1{foo}px + 5{bar} - 12{baz}%)");
        assert_eq!(mt!(1 px - 5 u + 12 %).to_string(), "(1px - 5 + 12%)");
        assert_eq!(mt!(1 px - 5 u - 12 %).to_string(), "(1px - 5 - 12%)");
        assert_eq!(mt!(1 px - 4 * load(ident) - 12 % + prev).to_string(), "(1px - 4*load(ident) - 12% + prev)");
        assert_eq!(mt!(1 px - {foo} * load(ident) - 12 % + prev).to_string(), "(1px - 1{foo}*load(ident) - 12% + prev)");
        assert_eq!(mt!(1 px - {bar} * load(ident) - 12 % + prev).to_string(), "(1px - 5{bar}*load(ident) - 12% + prev)");
        assert_eq!(mt!(-4 u + 12 px - -12 u + -4 * prev - -8 * prev).to_string(), "(-4 + 12px + 12 - 4*prev + 8*prev)");
        assert_eq!(mt!(foo = load(bar)).to_string(), "(foo = load(bar))");
        assert_eq!(mt!(foo = -4 u + 12 px - -12 u + -4 * prev - -8 * prev).to_string(), "(foo = -4 + 12px + 12 - 4*prev + 8*prev)");
    }

    #[test]
    fn test_size_measurements() {
        assert_eq!(smt!(foo = -4 u + 12 px - -12 u + -4 * prev - -8 * prev).to_string(), "(foo = -4 + 12px + 12 - 4*prev + 8*prev)");
        assert_eq!(smt!(auto).to_string(), "auto");
    }
}