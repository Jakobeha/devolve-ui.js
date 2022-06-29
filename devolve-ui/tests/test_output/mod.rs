//! Helpers for tests to compare prettified terminal output.
//! The output is written to a file and compared against another "expected" file,
//! so it can be viewed with terminal formatting via `less -r` or another command.

#![cfg(feature = "tui")]

use std::cell::RefCell;
use std::ffi::OsStr;
use std::fs::File;
use std::{env, io, thread};
use std::io::{ErrorKind, Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::rc::Rc;
use std::path::PathBuf;
use std::sync::{Arc, Weak as WeakArc, Mutex};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use devolve_ui::core::component::context::VComponentContext2;
use devolve_ui::core::component::node::VNode;
use devolve_ui::core::misc::notify_flag::NotifyFlag;
use devolve_ui::core::renderer::renderer::{Renderer, RendererOverrides};
use devolve_ui::core::view::layout::geom::Size;
use devolve_ui::engines::tui::tui::{TuiConfig, TuiEngine, TuiInputMode};
use devolve_ui::view_data::tui::tui::TuiViewData;
#[cfg(feature = "tui-images")]
use devolve_ui::view_data::tui::terminal_image::TuiImageFormat;

pub struct TestOutput {
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

pub struct ReadReciever(Receiver<u8>);

impl Read for ReadReciever {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut num = 0;
        loop {
            match self.0.try_recv() {
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => Err(io::Error::new(ErrorKind::BrokenPipe, TryRecvError::Disconnected))?,
                Ok(byte) => buf[num] = byte
            }
            num += 1;

        }
        Ok(num)
    }
}

#[allow(dead_code)]
pub fn assert_render_snapshot(
    test_name: &str,
    construct_root: impl Fn(VComponentContext2<(), TuiViewData>) -> VNode<TuiViewData> + 'static,
    adjust_config: impl FnOnce(&mut TuiConfig<io::Empty, TestOutput>),
    adjust_overrides: impl FnOnce(&mut RendererOverrides)
) {
    assert_render(
        test_name,
        construct_root,
        adjust_config,
        adjust_overrides,
        io::empty(),
        |_renderer| {}
    )
}

#[allow(dead_code)]
#[cfg(feature = "time-blocking")]
pub fn assert_render_multi(
    test_name: &str,
    construct_root: impl Fn(VComponentContext2<(), TuiViewData>) -> VNode<TuiViewData> + 'static,
    adjust_config: impl FnOnce(&mut TuiConfig<ReadReciever, TestOutput>),
    adjust_overrides: impl FnOnce(&mut RendererOverrides),
    run_in_background: impl FnOnce(Sender<u8>) + Send + 'static
) {
    let escape: Arc<Mutex<WeakArc<NotifyFlag>> >= Arc::new(Mutex::new(WeakArc::new()));
    let escape2 = escape.clone();

    let (tx, rx) = channel();
    thread::spawn(move || {
        let escape = escape2;

        run_in_background(tx);

        escape.lock().expect("renderer thread crashed").upgrade().expect("renderer already stopped").set();
    });

    assert_render(
        test_name,
        construct_root,
        adjust_config,
        adjust_overrides,
        ReadReciever(rx),
        |renderer| {
            renderer.resume_blocking_with_escape(|e| *escape.lock().expect("backtround thread crashed") = e)
        }
    )
}

fn assert_render<TestInput: Read + 'static>(
    test_name: &str,
    construct_root: impl Fn(VComponentContext2<(), TuiViewData>) -> VNode<TuiViewData> + 'static,
    adjust_config: impl FnOnce(&mut TuiConfig<TestInput, TestOutput>),
    adjust_overrides: impl FnOnce(&mut RendererOverrides),
    input: TestInput,
    adjust_renderer: impl FnOnce(&Rc<Renderer<TuiEngine<TestInput, TestOutput>>>)
) {
    let output = TestOutput::new();
    let mut config = TuiConfig {
        input,
        output: output.clone(),
        input_mode: TuiInputMode::ReadAscii,
        #[cfg(target_family = "unix")]
        termios_fd: None,
        output_ansi_escapes: true,
        #[cfg(feature = "tui-images")]
        image_format: TuiImageFormat::FallbackColor
    };
    adjust_config(&mut config);

    let mut overrides = RendererOverrides {
        override_size: Some(Size { width: 80f32, height: 40f32 }),
        ignore_events: true,
        ..RendererOverrides::default()
    };
    adjust_overrides(&mut overrides);

    let renderer = Renderer::new_with_overrides(TuiEngine::new(config), overrides);
    renderer.root(construct_root);
    // renderer.interval_between_frames(Duration::from_millis(25)); // optional
    renderer.show();
    adjust_renderer(&renderer);

    let test_output_dir = PathBuf::from(format!("{}/test-output/snapshots", env!("CARGO_MANIFEST_DIR")));
    assert!(test_output_dir.exists(), "test output dir doesn't exist! {}", test_output_dir.display());
    let actual_dir = PathBuf::from(format!("{}/{}-actual.txt", test_output_dir.display(), test_name));
    let expected_dir = PathBuf::from(format!("{}/{}-expected.txt", test_output_dir.display(), test_name));
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