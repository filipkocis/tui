#[cfg(feature = "console_logger")]
pub mod logger;

use std::{
    collections::VecDeque,
    sync::{LazyLock, RwLock},
    time::Duration,
};

use crossterm::event::{KeyCode, KeyModifiers};

use crate::{text::Text, *};

pub struct Console;

/// Entry in console [history](HISTORY)
enum Entry {
    Info(String),
    Warn(String),
    Error(String),
}

impl Entry {
    /// Returns the [text](Text) representation of [self](Entry)
    pub fn as_text(&self) -> Text {
        match self {
            Self::Info(t) => t.as_str().into(),
            Self::Warn(t) => t.as_str().into(),
            Self::Error(t) => t.as_str().into(),
        }
    }
}

/// History for [Console]
struct History {
    store: VecDeque<Entry>,
    version: usize,
    size: u16,
}

impl History {
    /// New Console history
    fn new() -> Self {
        Self {
            store: VecDeque::default(),
            version: 0,
            // TODO: scrolling
            // size: 1_000,
            size: 38,
        }
    }

    /// Ticks the version
    fn tick(&mut self) {
        self.version = self.version.wrapping_add(1);
    }
}

/// Global Console History
static HISTORY: LazyLock<RwLock<History>> = LazyLock::new(|| RwLock::new(History::new()));

/// Push a new `entry` to Console [HISTORY]
fn push_history(entry: Entry) {
    if let Ok(mut history) = HISTORY.write() {
        history.store.push_back(entry);
        history.tick();

        let new_start = history.store.len().saturating_sub(history.size as usize);

        if new_start > 0 {
            history.store.drain(0..new_start);
        }
    }
}

impl Console {
    /// Registers console `open/close` toggling with `toggle` key press.
    /// Registering multiple times will cause unexpected behaviour since previous handlers on
    /// `node` do not get removed automatically.
    ///
    /// `node` is usually the root, but can be any node on which the `capturing` toggle event
    /// handler will be created
    pub fn register_toggle(console: WeakNodeHandle, node: &mut Node, toggle: KeyCode) {
        let mut last_size = Size::default();
        let mut last_border = (false, false, false, false, None);
        let mut opened = true;

        let on_press = move |c: &mut Context, _: &mut Node| -> bool {
            let Some(node) = console.upgrade() else {
                return false;
            };
            let Ok(mut node) = node.try_borrow_mut() else {
                return false;
            };

            if let Some(key_event) = c.event.as_key_press_event() {
                if key_event.code == toggle {
                    if opened {
                        last_size = node.style.size;
                        last_border = node.style.border;

                        node.style.size = Size::from_cells(0, 0);
                        node.style.border = (false, false, false, false, None);
                        c.app.emmit(Action::FocusNode(c.self_weak.clone()));
                    } else {
                        node.style.size = last_size;
                        node.style.border = last_border;

                        if let Some(console_input) = node.children.get(2) {
                            c.app.emmit(Action::FocusNode(console_input.weak()));
                        }
                    }

                    opened = !opened;
                    c.app.emmit(Action::RecomputeNode(console.clone()));
                    return true;
                }
            }
            return false;
        };
        node.add_handler(on_press, true);
    }

    /// Creates a new console, combine with [Console::register_toggle] to get better interactivity.
    /// Each console instance will have the same output.
    /// `refresh_rate_hz` specifies the refresh rate of the Console output per second
    pub fn new(refresh_rate_hz: f32) -> NodeHandle {
        const WIDTH: u16 = 60;
        const HEIGHT: u16 = 40;

        let mut root = Draggable::new(None, Some((0, 1)), KeyModifiers::NONE);
        let root_id = root.id();
        root.style.border = (true, true, true, true, None);
        root.style.offset = Offset::Absolute(0, 0);
        root.style.size = Size::from_cells(WIDTH, HEIGHT);
        root.style.bg = Some(Hsl::new(0.0, 0.0, 0.2).into());

        // Window bar
        let mut window_bar = Node::default();
        window_bar.style.size = Size::parse("100%", "1").unwrap();
        window_bar.style.bg = Some(Hsl::new(0.0, 0.0, 0.3).into());
        window_bar.style.gap = (1, 0);
        window_bar.style.flex_row = true;
        window_bar.style.justify = Justify::SpaceBetween;

        let mut label = Node::default();
        label.text = "Console".into();

        let mut close_button = Button::new(
            "x",
            Some(Box::new(move |c, _, _| {
                c.app.emmit(Action::RemoveNode(root_id));
                return true;
            })),
        );
        close_button.style.size = Size::from_cells(1, 1);
        close_button.style.padding = (0, 1).into();
        close_button.style.bg = Some(Hsl::new(10., 1., 0.5).into());

        let window_bar = window_bar.into_handle();
        window_bar.add_child_node(label);
        window_bar.add_child_node(close_button);

        // History log
        let mut history = Node::default();
        history.style.size = Size::new(SizeValue::percent(100), SizeValue::cells(HEIGHT - 2));
        history.start_worker(move |c| {
            let mut last_seen_version = 0;

            while !c.is_shutdown() {
                std::thread::sleep(Duration::from_secs_f32(1.0 / refresh_rate_hz));
                let version = HISTORY.read().unwrap().version;
                if version <= last_seen_version {
                    continue;
                }
                last_seen_version = version;

                c.send(Message::exec(|mut c| {
                    let self_weak = c.self_weak.clone();

                    if let Ok(history) = HISTORY.read() {
                        let mut node = c.node_mut();
                        node.children.clear();

                        for entry in history.store.iter() {
                            let mut entry_node = Node::default();
                            entry_node.style.max_size = Size::parse("100%", "auto").unwrap();
                            entry_node.text = entry.as_text();
                            // TODO: FIX: if color is None make it inherit parent
                            entry_node.style.bg = Some(Hsl::new(0.0, 0.0, 0.2).into());
                            node.add_child(entry_node.into_handle(), self_weak.clone())
                        }
                    }

                    c.app().emmit(Action::RecomputeNode(self_weak));
                }))
                .ok()
                .unwrap();
            }
        });

        // Input field
        let mut input = Input::new(" >");
        input.style.size = Size::parse("100%", "1").unwrap();
        input.style.bg = Some(Hsl::new(0.0, 0.0, 0.3).into());

        // Combine window
        let root = root.into_handle();
        root.add_child(window_bar);
        root.add_child_node(history);
        root.add_child_node(input);
        root
    }

    /// Sets the max console history size
    #[inline]
    pub fn set_history_size(size: u16) {
        if let Ok(mut history) = HISTORY.write() {
            history.size = size;
            history.tick();
        }
    }

    /// Logs a message to console
    #[inline]
    pub fn log(text: impl Into<String>) {
        push_history(Entry::Info(text.into()))
    }
}
