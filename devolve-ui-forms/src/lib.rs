#![feature(decl_macro)]

use std::any::Any;
use std::borrow::Cow;
use std::collections::BTreeSet;
use std::marker::PhantomData;
#[allow(unused_imports)]
use devolve_ui::core::component::constr::{_make_component_macro, make_component};
use devolve_ui::core::component::context::{VComponentContext1, VComponentContext2, VContext, VContextIndex, VEffectContext2};
use devolve_ui::core::component::node::VNode;
use devolve_ui::core::hooks::provider::{ProviderIdSource, ProvidedState};
use devolve_ui::core::hooks::BuiltinHooks;
use devolve_ui::core::hooks::state::State;
use devolve_ui::core::misc::shorthand::d;
use devolve_ui::core::renderer::input::{KeyCode, KeyModifiers};
use devolve_ui::core::view::layout::macros::{mt, smt};
use devolve_ui::core::view::view::VViewData;
use devolve_ui::view_data::attrs::BorderStyle;
use devolve_ui::view_data::tui::constr::*;
use devolve_ui::view_data::tui::tui::HasTuiViewData;

make_component!(pub focus_provider, FocusProvider<ViewData: VViewData + Clone + 'static> {
    enable_tab: bool = true,
    _p: PhantomData<ViewData> = PhantomData
} [content: Box<dyn Fn(VComponentContext1<FocusProvider<ViewData>, ViewData>) -> VNode<ViewData>>]);

make_component!(pub text_field, TextField<Props: Any, ViewData: VViewData + HasTuiViewData> {
    initial_value: Cow<'static, str> = "".into(),
    placeholder: Cow<'static, str> = "".into(),
    is_enabled: bool = true,
    override_value: Option<String> = None,
    on_change: Option<Box<dyn Fn(VEffectContext2<Props, ViewData>, &str)>> = None,
    _p: PhantomData<(Props, ViewData, )> = PhantomData
} []);

#[derive(Default)]
pub struct FocusContext {
    pub focusable_ids: BTreeSet<usize>,
    pub next_free_id: usize,
    pub focused_id: Option<usize>
}

pub struct LocalFocus<ViewData: VViewData> {
    focus_context: ProvidedState<FocusContext, ViewData>,
    my_id: State<usize, ViewData>
}

impl <ViewData: VViewData> LocalFocus<ViewData> {
    fn is_focused<'a>(&self, c: &mut impl VContext<'a, ViewData=ViewData>) -> bool {
        self.focus_context.get(c).focused_id == Some(*self.my_id.get(c))
    }

    fn focus<'a>(&mut self, c: &mut impl VContext<'a, ViewData=ViewData>) {
        self.focus_context.get_mut(c).focused_id = Some(*self.my_id.get(c))
    }
}

static FOCUS_PROVIDER_CONTEXT: ProviderIdSource<FocusContext> = ProviderIdSource::new();

pub fn focus_provider<ViewData: VViewData + Clone + 'static>((mut c, FocusProvider {
    content,
    enable_tab,
    _p
}): VComponentContext2<FocusProvider<ViewData>, ViewData>) -> VNode<ViewData> {
    let focus_context = c.use_provide(&FOCUS_PROVIDER_CONTEXT, |_| Box::new(FocusContext::default()));

    c.use_key_listener_when(*enable_tab, move |(mut c, FocusProvider { content, enable_tab, _p }), event| {
        match event.code {
            KeyCode::Tab => {
                let focus_context = focus_context.get_mut(&mut c);
                if let Some(id) = focus_context.focused_id {
                    focus_context.focused_id = focus_context.focusable_ids.iter().skip_while(|&&id2| id2 <= id).next().copied();
                }
                if focus_context.focused_id.is_none() {
                    focus_context.focused_id = focus_context.focusable_ids.iter().next().copied();
                }
            }
            KeyCode::BackTab => {
                let focus_context = focus_context.get_mut(&mut c);
                if let Some(id) = focus_context.focused_id {
                    focus_context.focused_id = focus_context.focusable_ids.iter().rev().skip_while(|&&id2| id2 >= id).next().copied();
                }
                if focus_context.focused_id.is_none() {
                    focus_context.focused_id = focus_context.focusable_ids.iter().next_back().copied();
                }
            }
            _ => {}
        }
    });

    content(c)
}

pub fn use_focus<Props: Any, ViewData: VViewData + 'static>(c: &mut VComponentContext1<Props, ViewData>) -> LocalFocus<ViewData> {
    let focus_context = c.use_consume(&FOCUS_PROVIDER_CONTEXT);
    let my_id = c.use_state(|c| {
        let is_first = focus_context.get(c).focusable_ids.is_empty();
        let my_id = focus_context.get(c).next_free_id;
        focus_context.get_mut(c).next_free_id += 1;
        focus_context.get_mut(c).focusable_ids.insert(my_id);

        // Focus if this is the first focusable element
        if is_first {
            focus_context.get_mut(c).focused_id = Some(my_id);
        }

        my_id
    });

    LocalFocus { focus_context, my_id }
}

pub fn text_field<Props: Any, ViewData: HasTuiViewData + 'static>((mut c, TextField { initial_value, placeholder, is_enabled, override_value, on_change, _p }): VComponentContext2<TextField<Props, ViewData>, ViewData>) -> VNode<ViewData> {
    let mut focus = use_focus(&mut c);
    let mut value = c.use_state(|_| initial_value.to_string());
    let mut cursor = c.use_state(|_| 0);
    let mut is_focused = focus.is_focused(&mut c);

    c.use_key_listener_when(*is_enabled && is_focused, move |(mut c, props), key| {
        if KeyModifiers::SHIFT.contains(key.modifiers) {
            let cursor_ = *cursor.get(&c);
            let is_shift = key.modifiers.contains(KeyModifiers::SHIFT);
            match key.code {
                KeyCode::Backspace => {
                    if cursor_ != 0 {
                        value.get_mut(&mut c).remove(cursor_ - 1);
                        *cursor.get_mut(&mut c) -= 1;
                    }
                }
                KeyCode::Delete => {
                    if cursor_ < value.get(&c).len() {
                        value.get_mut(&mut c).remove(cursor_);
                    }
                }
                KeyCode::Left => {
                    if cursor_ > 0 {
                        *cursor.get_mut(&mut c) -= 1;
                    }
                }
                KeyCode::Right => {
                    if cursor_ < value.get(&c).len() {
                        *cursor.get_mut(&mut c) += 1;
                    }
                }
                KeyCode::Down => {
                    if cursor_ < value.get(&c).len() {
                        *cursor.get_mut(&mut c) = value.get(&c).len();
                    }
                }
                KeyCode::Up => {
                    if cursor_ > 0 {
                        *cursor.get_mut(&mut c) = 0;
                    }
                }
                KeyCode::CharAsLowercase(mut char) => {
                    if is_shift {
                        char = char.to_uppercase().next().unwrap();
                    }
                    value.get_mut(&mut c).insert(cursor_, char);
                    *cursor.get_mut(&mut c) += 1;
                }
                _ => {}
            }
        }
    });

    let mut txt = override_value.as_ref().unwrap_or_else(|| value.get(&c)).clone();
    let cursor = *cursor.get(&c);
    if cursor < txt.len() {
        txt.remove(cursor);
        txt.insert(cursor, '█');
    } else {
        txt.push('█');
    }

    zbox(Vvw1 {
        width: smt!(16 u),
        height: smt!(3 u),
        ..d()
    }, d(), vec![
        text(Vvw1 {
            x: mt!(1 u),
            y: mt!(1 u),
            width: smt!(100% - 2 u),
            height: smt!(1 u),
            ..d()
        }, d(), txt),
        border(Vvw1 {
            width: smt!(100 %),
            height: smt!(100 %),
            ..d()
        }, d(), BorderStyle::Rounded)
    ])
}

#[cfg(test)]
mod test {
    use std::io;
    use std::io::{ErrorKind, Read};
    use std::thread;
    use std::sync::{Arc, Mutex, Weak as WeakArc};
    use std::sync::mpsc::{channel, Receiver, TryRecvError};
    use std::time::Duration;
    #[allow(unused_imports)]
    use devolve_ui::core::component::constr::{_make_component_macro, make_component};
    use devolve_ui::core::component::context::{VComponentContext1, VComponentContext2, VEffectContext2};
    use devolve_ui::core::component::node::VNode;
    use devolve_ui::core::misc::notify_flag::NotifyFlag;
    use devolve_ui::core::renderer::renderer::Renderer;
    use devolve_ui::core::view::layout::macros::{mt, smt};
    use devolve_ui::engines::tui::tui::{TuiConfig, TuiEngine, TuiInputMode};
    use devolve_ui::view_data::tui::constr::*;
    use devolve_ui::view_data::attrs::BorderStyle;
    #[cfg(feature = "tui-images")]
    use devolve_ui::view_data::tui::terminal_image::TuiImageFormat;
    use devolve_ui::view_data::tui::tui::HasTuiViewData;
    use crate::{FocusProvider, focus_provider, text_field};
    use test_log::test;

    make_component!(test_app, TestApp {} []);

    struct ReadReciever(Receiver<u8>);

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
    pub fn test() {
        let mut escape: Arc<Mutex<WeakArc<NotifyFlag>> >= Arc::new(Mutex::new(WeakArc::new()));
        let escape2 = escape.clone();

        let (tx, rx) = channel();
        thread::spawn(move || {
            let escape = escape2;

            thread::sleep(Duration::from_secs(5));
            tx.send(b'h').unwrap();
            thread::sleep(Duration::from_secs(1));
            tx.send(b'e').unwrap();
            thread::sleep(Duration::from_secs(1));
            tx.send(b'l').unwrap();
            thread::sleep(Duration::from_secs(1));
            tx.send(b'l').unwrap();
            thread::sleep(Duration::from_secs(1));
            tx.send(b'o').unwrap();
            thread::sleep(Duration::from_secs(5));
            tx.send(b'\t').unwrap();
            thread::sleep(Duration::from_secs(5));
            tx.send(b'w').unwrap();
            thread::sleep(Duration::from_secs(1));
            tx.send(b'o').unwrap();
            thread::sleep(Duration::from_secs(1));
            tx.send(b'r').unwrap();
            thread::sleep(Duration::from_secs(1));
            tx.send(b'l').unwrap();
            thread::sleep(Duration::from_secs(1));
            tx.send(b'd').unwrap();
            thread::sleep(Duration::from_secs(5));
            tx.send(b'?').unwrap();
            thread::sleep(Duration::from_secs(1));
            tx.send(b'?').unwrap();
            thread::sleep(Duration::from_secs(1));
            tx.send(b'\x08').unwrap();
            thread::sleep(Duration::from_secs(1));
            tx.send(b'\x08').unwrap();
            thread::sleep(Duration::from_secs(1));
            tx.send(b'!').unwrap();
            thread::sleep(Duration::from_secs(1));
            tx.send(b'!').unwrap();
            thread::sleep(Duration::from_secs(5));

            escape.lock().expect("renderer thread crashed").upgrade().expect("renderer already stopped").set();
        });

        let renderer = Renderer::new(TuiEngine::new(TuiConfig {
            input: ReadReciever(rx),
            output: io::stdout(),
            input_mode: TuiInputMode::ReadAscii,
            #[cfg(target_family = "unix")]
            termios_fd: None,
            output_ansi_escapes: true,
            #[cfg(feature = "tui-images")]
            image_format: TuiImageFormat::FallbackColor
        }));
        renderer.root(|(mut c, ())| test_app!(c, (), {}));
        renderer.show();
        renderer.resume_blocking_with_escape(|e| *escape.lock().unwrap() = e);
    }
}