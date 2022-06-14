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
use devolve_ui::core::component::constr::{_make_component2, make_component2};
use devolve_ui::core::component::context::VComponentContext2;
use devolve_ui::core::component::node::VNode;
use devolve_ui::core::renderer::renderer::{Renderer, RendererOverrides};
use devolve_ui::core::view::layout::geom::Size;
use devolve_ui::engines::tui::tui::{TuiConfig, TuiEngine};
use devolve_ui::view_data::tui::tui::TuiViewData;
use devolve_ui::view_data::tui::constr::{vbox, text};

#[derive(Default)]
pub struct BasicProps {
    text: String
}

pub fn basic((_c, props): VComponentContext2<BasicProps, TuiViewData>) -> VNode<TuiViewData> {
    vbox!({}, {}, vec![
        text!({}, {}, "Hello world!".to_string()),
        text!({}, {}, props.text.clone())
    ])
}

make_component2!(pub basic, basic, BasicProps);

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
fn test_basic_render() {
    let output = TestOutput::new();
    let renderer = Renderer::new_with_overrides(TuiEngine::new(TuiConfig {
        input: io::empty(),
        output: output.clone(),
        raw_mode: true,
        #[cfg(target_family = "unix")]
        termios_fd: None
    }), RendererOverrides {
        override_size: Some(Size { width: 80f32, height: 40f32 }),
        override_column_size: Some(Size {
            width: 6f32,
            height: 12f32
        }),
        additional_store: Default::default(),
        ignore_events: false
    });
    renderer.root(|(mut c, ())| basic!(&mut c, "basic", { text: "foo bar".into() }));
    // renderer.interval_between_frames(Duration::from_millis(25)); // optional
    renderer.show();
    // renderer.resume();
    // TODO: Windows support
    assert_eq!(
        OsStr::from_bytes(&output.snapshot_buf()),
        OsStr::new("\u{1b}[?1049h\u{1b}[2J\u{1b}[25l\u{1b}[1;1HHello world!\u{1b}[0m\u{1b}[2;1Hfoo bar     \u{1b}[0m")
    )
}