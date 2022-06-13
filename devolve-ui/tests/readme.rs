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
use devolve_ui::core::component::constr::{_make_component, _make_component2, make_component};
use devolve_ui::core::renderer::renderer::{Renderer, RendererOverrides};
use devolve_ui::core::hooks::state::use_state;
use devolve_ui::core::view::color::Color;
use devolve_ui::core::view::layout::geom::Size;
use devolve_ui::core::view::layout::macros::{mt, smt};
use devolve_ui::engines::tui::tui::{TuiConfig, TuiEngine};
use devolve_ui::view_data::tui::tui::TuiViewData;
use devolve_ui::view_data::tui::terminal_image::Source;
use devolve_ui::view_data::attrs::BorderStyle;
use devolve_ui::view_data::tui::constr::{border, hbox, source, text, zbox};
use devolve_ui::core::hooks::event::{CallFirst, use_interval};

make_component!(
    header,
    HeaderProps {
        name: String
    },
    {
        name: Default::default()
    },
    <TuiViewData>|c, name| {
        let counter = use_state(c, || 0);
        use_interval(c, Duration::from_secs(1), CallFirst::AfterTheInterval, move |c| {
            *counter.get_mut(c) += 1;
        });

        zbox!({ width: smt!(100%) }, {}, vec![
            zbox!({ x: mt!(1), y: mt!(1), width: smt!(32) }, {}, vec![
                text!({}, { color: Some(Color::yellow()) }, format!("Hello {}", name)),
                text!({ x: mt!(100%), anchor_x: 1f32 }, { color: Some(Color::yellow()) }, format!("{} seconds", counter.get(c)))
            ]),
            border!({ width: smt!(34), height: smt!(prev + 2) }, { color: Some(Color::yellow()) }, BorderStyle::Rounded)
        ])
    }
);

make_component!(
    pub readme,
    ReadmeProps {
        name: String
    },
    {
        name: Default::default()
    },
    <TuiViewData>|c, name| {
        zbox!({ width: smt!(100%) }, {}, vec![
            hbox!({ x: mt!(2), y: mt!(1), width: smt!(100% - 4) }, { gap: mt!(1) }, vec![
                header!(c, "header", { name: name.clone() }),
                source!({ width: smt!(34), height: smt!(16) }, {}, Source::Path(PathBuf::from("assets/dog.png")))
            ]),
            border!({ width: smt!(100%), height: smt!(prev + 2) }, { color: Some(Color::blue()) }, BorderStyle::Rounded)
        ])
    }
);

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
    }), RendererOverrides {
        override_size: Some(Size { width: 80f32, height: 40f32 }),
        ignore_events: true,
        ..RendererOverrides::default()
    });
    renderer.root(|c| readme!(c, "readme", { name: "devolve-ui".into() }));
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
        OsStr::from_bytes(&expected)
    );
}