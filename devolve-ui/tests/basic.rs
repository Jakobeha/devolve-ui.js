//! Tests a really basic component and renderer.
//! Just to make sure there isn't something very wrong.

#![feature(decl_macro)]
#![feature(macro_metavar_expr)]
#![cfg(feature = "tui")]

use std::cell::RefCell;
use std::ffi::OsStr;
use std::io;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::rc::Rc;
#[allow(unused_imports)] // Needed for IntelliJ macro expansion
use devolve_ui::core::component::constr::{_make_component_macro, make_component_macro};
use devolve_ui::core::misc::partial_default::PartialDefault;
use devolve_ui::core::component::context::VComponentContext2;
use devolve_ui::core::component::node::VNode;
use devolve_ui::core::renderer::renderer::{Renderer, RendererOverrides};
use devolve_ui::core::view::layout::geom::Size;
use devolve_ui::engines::tui::tui::{TuiConfig, TuiEngine, TuiInputMode};
use devolve_ui::view_data::tui::tui::HasTuiViewData;
use devolve_ui::view_data::tui::constr::{vbox, text};
use devolve_ui::view_data::tui::terminal_image::TuiImageFormat;
use test_log::test;

pub struct BasicProps {
    pub text: String
}

impl PartialDefault for BasicProps {
    type RequiredArgs = (String,);

    fn partial_default((text,): Self::RequiredArgs) -> Self {
        Self {
            text
        }
    }
}

pub fn basic<ViewData: HasTuiViewData>((_c, props): VComponentContext2<BasicProps, ViewData>) -> VNode<ViewData> {
    vbox!({}, {}, vec![
        text!({}, {}, "Hello world!".to_string()),
        text!({}, {}, props.text.clone())
    ])
}

make_component_macro!(pub basic, basic, BasicProps);

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

fn test_basic_render(output_ansi_escapes: bool, expected_output: &str) {
    let output = TestOutput::new();
    let renderer = Renderer::new_with_overrides(TuiEngine::new(TuiConfig {
        input: io::empty(),
        output: output.clone(),
        input_mode: TuiInputMode::ReadAscii,
        output_ansi_escapes,
        #[cfg(target_family = "unix")]
        termios_fd: None,
        image_format: TuiImageFormat::FallbackColor
    }), RendererOverrides {
        override_size: Some(Size { width: 80f32, height: 40f32 }),
        override_column_size: Some(Size {
            width: 6f32,
            height: 12f32
        }),
        additional_store: Default::default(),
        ignore_events: false
    });
    renderer.root(|(mut c, ())| basic!(c, "basic", {}, "foo bar".into()));
    // renderer.interval_between_frames(Duration::from_millis(25)); // optional
    renderer.show();
    // renderer.resume();
    // TODO: Windows support
    assert_eq!(
        OsStr::from_bytes(&output.snapshot_buf()),
        OsStr::new(&expected_output),
        "actual != expected"
    )
}

#[test]
fn test_basic_render_no_ansi() {
    test_basic_render(false, "Hello world!\nfoo bar     \n");
}

#[test]
fn test_basic_render_ansi() {
    test_basic_render(true, "\u{1b}[?1049h\u{1b}[2J\u{1b}[25l\u{1b}[1;1HHello world!\u{1b}[0m\n\u{1b}[2;1Hfoo bar     \u{1b}[0m\n");
}