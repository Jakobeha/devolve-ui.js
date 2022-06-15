//! Tests a more-complex example which is (will be) used in the README.

#![feature(decl_macro)]
#![feature(macro_metavar_expr)]
#![cfg(feature = "tui")]

use std::cell::RefCell;
use std::ffi::OsStr;
use std::fs::File;
use std::{env, io};
use std::io::{Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::rc::Rc;
use std::path::PathBuf;
use std::time::Duration;
#[allow(unused_imports)] // Needed for IntelliJ macro expansion
use devolve_ui::core::component::constr::{_make_component_macro, make_component};
use devolve_ui::core::component::context::VComponentContext2;
use devolve_ui::core::component::node::VNode;
use devolve_ui::core::renderer::renderer::{Renderer, RendererOverrides};
use devolve_ui::core::hooks::state::use_state;
use devolve_ui::core::view::color::Color;
use devolve_ui::core::view::layout::geom::Size;
use devolve_ui::core::view::layout::macros::{mt, smt};
use devolve_ui::engines::tui::tui::{TuiConfig, TuiEngine};
use devolve_ui::view_data::tui::tui::TuiViewData;
use devolve_ui::view_data::tui::terminal_image::{Source, HandleAspectRatio, TuiImageFormat};
use devolve_ui::view_data::attrs::BorderStyle;
use devolve_ui::view_data::tui::constr::{border, hbox, source, text, zbox};
use devolve_ui::core::hooks::event::{CallFirst, use_interval};

make_component!(pub header, HeaderProps {} [ name: String ]);
make_component!(pub readme, ReadmeProps {
    name: String = "".to_string()
} []);

pub fn header((mut c, HeaderProps { name }): VComponentContext2<HeaderProps, TuiViewData>) -> VNode<TuiViewData> {
    let counter = use_state(&mut c, || 0);
    use_interval(&mut c, Duration::from_secs(1), CallFirst::AfterTheInterval, move |(mut c, HeaderProps { .. })| {
        *counter.get_mut(&mut c) += 1;
    });

    zbox!({ width: smt!(34) }, {}, vec![
        zbox!({ x: mt!(2), y: mt!(1), width: smt!(100% - 4) }, {}, vec![
            text!({}, { color: Some(Color::yellow()) }, format!("Hello {}", name)),
            text!({ x: mt!(100%), anchor_x: 1f32 }, { color: Some(Color::yellow()) }, format!("{} seconds", counter.get(&mut c)))
        ]),
        border!({ width: smt!(100%), height: smt!(prev) }, { color: Some(Color::yellow()) }, BorderStyle::Rounded)
    ])
}

pub fn readme((mut c, ReadmeProps { name }): VComponentContext2<ReadmeProps, TuiViewData>) -> VNode<TuiViewData> {
    zbox!({ width: smt!(100%) }, {}, vec![
        hbox!({ x: mt!(2), y: mt!(1), width: smt!(100% - 4) }, { gap: mt!(1) }, vec![
            header!(&mut c, "header", {}, name.clone()),
            source!({ width: smt!(34) }, { handle_aspect_ratio: HandleAspectRatio::Stretch }, Source::Path(PathBuf::from(format!("{}/test-resources/assets/dog.png", env!("CARGO_MANIFEST_DIR")))))
        ]),
        border!({ width: smt!(100%), height: smt!(prev) }, { color: Some(Color::blue()) }, BorderStyle::Rounded)
    ])
}

struct TestOutput {
    buf: Rc<RefCell<Vec<u8>>>
}

impl TestOutput {
    fn new() -> Self {
        Self {
            buf: Rc::new(RefCell::new(Vec::new()))
        }
    }

    fn snapshot_buf(&self) -> Vec<u8> {
        self.buf.borrow().clone()
    }
}

impl Clone for TestOutput {
    fn clone(&self) -> Self {
        Self {
            buf: self.buf.clone()
        }
    }
}

impl Write for TestOutput {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buf.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_render() {
    let output = TestOutput::new();
    let renderer = Renderer::new_with_overrides(TuiEngine::new(TuiConfig {
        input: io::empty(),
        output: output.clone(),
        raw_mode: true,
        #[cfg(target_family = "unix")]
        termios_fd: None,
        image_format: TuiImageFormat::Fallback
    }), RendererOverrides {
        override_size: Some(Size { width: 80f32, height: 40f32 }),
        ignore_events: true,
        ..RendererOverrides::default()
    });
    renderer.root(|(mut c, ())| readme!(&mut c, "readme", { name: "devolve-ui".into() }));
    // renderer.interval_between_frames(Duration::from_millis(25)); // optional
    renderer.show();
    // renderer.resume();

    // TODO generalize how we do this
    let mut test_output_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_output_dir.push("test-output");
    assert!(test_output_dir.exists(), "test output dir doesn't exist! {}", test_output_dir.display());
    let mut actual_dir = test_output_dir.clone();
    actual_dir.push("readme-actual.txt");
    let mut expected_dir = test_output_dir.clone();
    expected_dir.push("readme-expected.txt");
    let mut actual_file = File::options().write(true).create(true).open(actual_dir).expect("failed to open file for actual output");
    let mut expected_file = File::options().read(true).open(expected_dir).expect("failed to open expected output - create the file if it doesn't exist!");

    let actual = output.snapshot_buf();
    let mut expected = Vec::new();
    actual_file.write_all(&actual).expect("failed to write actual output");
    expected_file.read_to_end(&mut expected).expect("failed to read expected output");
    assert_eq!(
        OsStr::from_bytes(&actual),
        OsStr::from_bytes(&expected),
        "actual (left) != expected (right)"
    );
}