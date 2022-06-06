#![feature(decl_macro)]
#![feature(macro_metavar_expr)]
#![cfg(feature = "tui")]

use std::cell::RefCell;
use std::ffi::OsStr;
use std::io;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::rc::Rc;
use devolve_ui::core::component::constr::make_component;
use devolve_ui::core::renderer::renderer::Renderer;
use devolve_ui::core::view::layout::geom::Size;
use devolve_ui::core::view::layout::macros::{mt, smt};
use devolve_ui::engines::tui::tui::{TuiConfig, TuiEngine};
use devolve_ui::view_data::tui::tui::TuiViewData;
use devolve_ui::view_data::attrs::BorderStyle;
use devolve_ui::view_data::tui::constr::{border, source, vbox, text, zbox};

make_component!(
    header,
    HeaderProps {
        name: String
    },
    {
        name: Default::default()
    },
    <TuiViewData>|c, name| {
        let mut counter = use_state(c, || 0);
        use_interval(c, 1000, |c| {
            counter.get_mut(c) += 1;
        });

        zbox!({ width: smt!(100%) }, {}, vec![
            zbox!({ x: mt!(1), y: mt!(1), width: smt!(32) }, {}, vec![
                text!({}, { color: Color::yellow() }, format!("Hello {}", name)),
                text!({ x: mt!(100%), anchor_x: 1f32 }, { color: Color::yellow() }, format!("{} seconds", counter.get()))
            ]),
            border!({ width: smt!(34), height: smt!(prev + 2) }, { style: BorderStyle::Rounded, color: Color::yellow() })
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
            vbox!({ x: mt!(2), y: mt!(1), width: smt!(100% - 4) }, { gap: 1 }, vec![
                header!(c, "header", { name: name }),
                source!({ width: mt!(34), height: mt!(16) }, { path: "assets/dog.png" })
            ]),
            border!({ width: smt!(100%), height: smt!(prev + 2) }, { style: BorderStyle::Rounded, color: Color::blue() })
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
fn test_wordle_render() {
    let output = TestOutput::new();
    let renderer = Renderer::new(TuiEngine::new(TuiConfig {
        input: io::empty(),
        output: output.clone(),
        raw_mode: true,
        #[cfg(target_family = "unix")]
        termios_fd: None,
    }));
    renderer.root(|c| wordle!(c, "wordle", { text: "Hello world".into() }));
    // renderer.interval_between_frames(Duration::from_millis(25)); // optional
    renderer.show();
    // renderer.resume();
    // TODO: Windows support
    assert_eq!(
        OsStr::from_bytes(&output.snapshot_buf()),
        OsStr::new("\u{1b}[?1049h\u{1b}[2J\u{1b}[25l\u{1b}[1;1HHello world!\u{1b}[0m")
    )
}