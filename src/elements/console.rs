use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicU16, Ordering}, RwLock
    },
    time::{Duration, Instant},
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

// TODO: add some history version for optimized worker thread ?
static HISTORY: RwLock<VecDeque<Entry>> = RwLock::new(VecDeque::new());
// static HISTORY_SIZE: AtomicU16 = AtomicU16::new(1_000);
// TODO: scrolling
static HISTORY_SIZE: AtomicU16 = AtomicU16::new(38);

fn push_history(entry: Entry) {
    if let Ok(mut lock) = HISTORY.write() {
        lock.push_back(entry);

        let history_size = HISTORY_SIZE.load(Ordering::Relaxed);
        let new_start = lock.len().saturating_sub(history_size as usize);

        if new_start > 0 {
            lock.drain(0..new_start);
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
    /// Each console instance will have the same output
    /// # Note
    /// `refresh_rate` is the rate in milliseconds for the console output to refresh
    pub fn new(refresh_rate: u64) -> NodeHandle {
        const WIDTH: u16 = 60;
        const HEIGHT: u16 = 40;

        let mut root = Draggable::new(None, Some((0, 1)), KeyModifiers::NONE);
        root.style.border = (true, true, true, true, None);
        root.style.offset = Offset::Absolute(0, 0);
        root.style.size = Size::from_cells(WIDTH, HEIGHT);
        root.style.bg = Some(Hsl::new(0.0, 0.0, 0.2).into());

        // Window bar
        let mut window_bar = Node::default();
        window_bar.style.size = Size::parse("100%", "1").unwrap();
        window_bar.style.bg = Some(Hsl::new(0.0, 0.0, 0.3).into());
        window_bar.style.gap = (WIDTH - (7 + 3), 0);
        window_bar.style.flex_row = true;

        let mut label = Node::default();
        label.text = "Console".into();

        let mut close_button = Button::new("x", None);
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
            while !c.is_shutdown() {
                c.send(Message::exec(|mut c| {
                    let self_weak = c.self_weak.clone();
                    let mut history = c.node_mut();
                    history.children.clear();

                    if let Ok(lock) = HISTORY.read() {
                        for entry in lock.iter() {
                            let mut entry_node = Node::default();
                            entry_node.style.max_size = Size::parse("100%", "auto").unwrap();
                            entry_node.text = entry.as_text();
                            // TODO: FIX: if color is None make it inherit parent
                            entry_node.style.bg = Some(Hsl::new(0.0, 0.0, 0.2).into());
                            history.add_child(entry_node.into_handle(), self_weak.clone())
                        }
                    }

                    drop(history);
                    c.app().emmit(Action::RecomputeNode(self_weak));
                }))
                .ok()
                .unwrap();

                std::thread::sleep(Duration::from_millis(refresh_rate));
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
    pub fn set_history_size(size: u16) {
        HISTORY_SIZE.store(size, Ordering::Relaxed)
    }

    /// Prints a message to console
    pub fn print(text: String) {
        push_history(Entry::Info(text))
    }

    /// Prints a warning message to console
    pub fn warn(text: String) {
        push_history(Entry::Warn(text))
    }

    /// Prints an error message to console
    pub fn error(text: String) {
        push_history(Entry::Error(text))
    }
}
