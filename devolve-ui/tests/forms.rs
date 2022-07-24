#![cfg(all(feature = "tui", feature = "time-blocking", feature = "input", feature = "logging"))]
#![feature(decl_macro)]

use std::thread;
use std::time::Duration;
#[allow(unused_imports)]
use devolve_ui::component::constr::{_make_component_macro, make_component};
use devolve_ui::component::context::{VComponentContext1, VComponentContext2, VEffectContext2};
use devolve_ui::component::node::VNode;
use devolve_ui::view::layout::macros::{mt, smt};
use devolve_ui::view_data::tui::constr::*;
use devolve_ui::view_data::attrs::BorderStyle;
use devolve_ui::view_data::tui::tui::HasTuiViewData;
use devolve_ui_forms::{FocusProvider, focus_provider, text_field};
use test_log::test;

mod test_output;

make_component!(test_app, TestApp {} []);

fn test_app<ViewData: HasTuiViewData + Clone + 'static>((mut c, TestApp {}): VComponentContext2<TestApp, ViewData>) -> VNode<ViewData> {
    zbox!({
        width: smt!(100 %),
        height: smt!(100 %)
    }, {}, vec![
        focus_provider!(c, (), {}, Box::new(move |mut c: VComponentContext1<'_, '_, FocusProvider<ViewData>, ViewData>| vbox!({
            x: mt!(4 u),
            y: mt!(2 u),
            width: smt!(100 % - 8 u),
            height: smt!(100 % - 4 u)
        }, {
            gap: mt!(1 u)
        }, vec![
            text_field!(c, 1, {
                initial_value: "".into(),
                placeholder: "field 1".into(),
                is_enabled: true,
                override_value: None,
                on_change: None as Option<Box<dyn Fn(VEffectContext2<TestApp, ViewData>, &str)>>
            }),
            text_field!(c, 2, {
                initial_value: "field 2".into(),
                placeholder: "field 2".into(),
                is_enabled: true,
                override_value: None,
                on_change: None as Option<Box<dyn Fn(VEffectContext2<TestApp, ViewData>, &str)>>
            }),
            text_field!(c, 3, {
                initial_value: "".into(),
                placeholder: "field 3".into(),
                is_enabled: true,
                override_value: None,
                on_change: None as Option<Box<dyn Fn(VEffectContext2<TestApp, ViewData>, &str)>>
            }),
            text_field!(c, 4, {
                initial_value: "".into(),
                placeholder: "field 4".into(),
                is_enabled: false,
                override_value: Some("override".into()),
                on_change: None as Option<Box<dyn Fn(VEffectContext2<TestApp, ViewData>, &str)>>
            })
        ])) as Box<dyn for<'r, 's> Fn(VComponentContext1<'r, 's, FocusProvider<ViewData>, ViewData>) -> VNode<ViewData> + 'static>),
        border!({
            width: smt!(100 %),
            height: smt!(100 %)
        }, {}, BorderStyle::Rounded)
    ])
}

#[test]
pub fn test_no_ansi() {
    test_output::assert_render_multi(
        "forms",
        |(mut c, ())| test_app!(c, (), {}),
        |config| {
            config.output_ansi_escapes = false;
        },
        |_overrides| {},
        |tx| {
            thread::sleep(Duration::from_millis(50));
            tx.send(b'h').unwrap();
            thread::sleep(Duration::from_millis(10));
            tx.send(b'e').unwrap();
            thread::sleep(Duration::from_millis(10));
            tx.send(b'l').unwrap();
            thread::sleep(Duration::from_millis(10));
            tx.send(b'l').unwrap();
            thread::sleep(Duration::from_millis(10));
            tx.send(b'o').unwrap();
            thread::sleep(Duration::from_millis(50));
            tx.send(b'\t').unwrap();
            thread::sleep(Duration::from_millis(50));
            tx.send(b'w').unwrap();
            thread::sleep(Duration::from_millis(10));
            tx.send(b'o').unwrap();
            thread::sleep(Duration::from_millis(10));
            tx.send(b'r').unwrap();
            thread::sleep(Duration::from_millis(10));
            tx.send(b'l').unwrap();
            thread::sleep(Duration::from_millis(10));
            tx.send(b'd').unwrap();
            thread::sleep(Duration::from_millis(50));
            tx.send(b'?').unwrap();
            thread::sleep(Duration::from_millis(10));
            tx.send(b'?').unwrap();
            thread::sleep(Duration::from_millis(10));
            tx.send(b'\x08').unwrap();
            thread::sleep(Duration::from_millis(10));
            tx.send(b'\x08').unwrap();
            thread::sleep(Duration::from_millis(10));
            tx.send(b'!').unwrap();
            thread::sleep(Duration::from_millis(10));
            tx.send(b'!').unwrap();
            thread::sleep(Duration::from_millis(50));
        }
    );
}