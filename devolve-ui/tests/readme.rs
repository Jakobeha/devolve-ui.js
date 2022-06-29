//! Tests a more-complex example which is (will be) used in the README.

#![feature(decl_macro)]
#![feature(macro_metavar_expr)]
#![cfg(feature = "tui")]

use std::env;
use std::path::PathBuf;
use std::time::Duration;
#[allow(unused_imports)] // Needed for IntelliJ macro expansion
use devolve_ui::core::component::constr::{_make_component_macro, make_component};
use devolve_ui::core::component::context::VComponentContext2;
use devolve_ui::core::component::node::VNode;
use devolve_ui::core::hooks::BuiltinHooks;
use devolve_ui::core::view::color::Color;
use devolve_ui::core::view::layout::macros::{mt, smt};
use devolve_ui::view_data::tui::tui::HasTuiViewData;
use devolve_ui::view_data::tui::terminal_image::{Source, HandleAspectRatio, TuiImageFormat};
use devolve_ui::view_data::attrs::BorderStyle;
use devolve_ui::view_data::tui::constr::{border, hbox, source, text, zbox};
use devolve_ui::core::hooks::event::CallFirst;

mod test_output;

make_component!(pub header, HeaderProps {} [ name: String ]);
make_component!(pub readme, ReadmeProps {
    name: String = "".to_string()
} []);

pub fn header<ViewData: HasTuiViewData + 'static>((mut c, HeaderProps { name }): VComponentContext2<HeaderProps, ViewData>) -> VNode<ViewData> {
    let counter = c.use_state(|_| 0);
    c.use_interval(Duration::from_secs(1), CallFirst::AfterTheInterval, move |(mut c, HeaderProps { .. })| {
        *counter.get_mut(&mut c) += 1;
    });

    zbox!({ width: smt!(34 u) }, {}, vec![
        zbox!({ x: mt!(2 u), y: mt!(1 u), width: smt!(100% - 4 u) }, {}, vec![
            text!({}, { color: Some(Color::yellow()) }, format!("Hello {}", name)),
            text!({ x: mt!(100%), anchor_x: 1f32 }, { color: Some(Color::yellow()) }, format!("{} seconds", counter.get(&mut c)))
        ]),
        border!({ width: smt!(100%), height: smt!(prev) }, { color: Some(Color::yellow()) }, BorderStyle::Rounded)
    ])
}

pub fn readme<ViewData: HasTuiViewData + 'static>((mut c, ReadmeProps { name }): VComponentContext2<ReadmeProps, ViewData>) -> VNode<ViewData> {
    zbox!({ width: smt!(100%) }, {}, vec![
        hbox!({ x: mt!(2 u), y: mt!(1 u), width: smt!(100% - 4 u) }, { gap: mt!(1 u) }, vec![
            header!(c, "header", {}, name.clone()),
            source!({ width: smt!(34 u) }, { handle_aspect_ratio: HandleAspectRatio::Stretch }, Source::Path(PathBuf::from(format!("{}/test-resources/assets/dog.png", env!("CARGO_MANIFEST_DIR")))))
        ]),
        border!({ width: smt!(100%), height: smt!(prev) }, { color: Some(Color::blue()) }, BorderStyle::Rounded)
    ])
}

#[test]
fn test_snapshot_with_grayscale_image_no_ansi() {
    test_output::assert_render_snapshot(
        "readme-grayscale",
        |(mut c, ())| readme!(c, "readme", { name: "devolve-ui".into() }),
        |mut config| {
            config.output_ansi_escapes = false;
            config.image_format = TuiImageFormat::FallbackGray;
        },
        |_overrides| {}
    );
}

#[test]
fn test_snapshot_color() {
    test_output::assert_render_snapshot(
        "readme",
        |(mut c, ())| readme!(c, "readme", { name: "devolve-ui".into() }),
        |_config| {},
        |_overrides| {}
    );
}

#[test]
fn test_snapshot_with_sixel() {
    test_output::assert_render_snapshot(
        "readme-sixel",
        |(mut c, ())| readme!(c, "readme", { name: "devolve-ui".into() }),
        |config| {
            config.image_format = TuiImageFormat::Sixel;
        },
        |_overrides| {}
    );
}