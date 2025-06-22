use crossterm::event::{KeyEvent, KeyEventKind, MouseEvent};

/// Represents an event. It's mostly a wrapper around [crossterm event](crossterm::event::Event),
/// but with additional events. For better documentation on some functions, see their crossterm
/// counterparts.
#[derive(Debug, PartialOrd, PartialEq, Eq, Clone, Hash)]
pub enum Event {
    /// The terminal gained focus
    TerminalFocusGained,
    /// The terminal lost focus
    TerminalFocusLost,
    /// A node lost focus
    NodeFocusLost,
    /// A node gained focus
    NodeFocusGained,
    /// A single key event with additional pressed modifiers.
    Key(KeyEvent),
    /// A single mouse event with additional pressed modifiers.
    Mouse(MouseEvent),
    /// A string that was pasted into the terminal. Only emitted if bracketed paste has been
    /// enabled.
    Paste(String),
    /// An resize event with new dimensions after resize (columns, rows).
    /// **Note** that resize events can occur in batches.
    TerminalResize(u16, u16),
}

impl Event {
    /// Transforms a [crossterm event](crossterm::event::Event) into an [`Event`].
    #[inline]
    pub fn from_crossterm_event(event: crossterm::event::Event) -> Self {
        use crossterm::event::Event as CrosstermEvent;
        match event {
            CrosstermEvent::FocusGained => Self::TerminalFocusGained,
            CrosstermEvent::FocusLost => Self::TerminalFocusLost,
            CrosstermEvent::Key(key_event) => Self::Key(key_event),
            CrosstermEvent::Mouse(mouse_event) => Self::Mouse(mouse_event),
            CrosstermEvent::Paste(string) => Self::Paste(string),
            CrosstermEvent::Resize(columns, rows) => Self::TerminalResize(columns, rows),
        }
    }

    /// Returns `true` if the event is a key press event.
    #[inline]
    pub fn is_key_press(&self) -> bool {
        matches!(
            self,
            Self::Key(KeyEvent {
                kind: KeyEventKind::Press,
                ..
            })
        )
    }

    /// Returns `true` if the event is a key release event.
    #[inline]
    pub fn is_key_release(&self) -> bool {
        matches!(
            self,
            Self::Key(KeyEvent {
                kind: KeyEventKind::Release,
                ..
            })
        )
    }

    /// Returns `true` if the event is a key repeat event.
    #[inline]
    pub fn is_key_repeat(&self) -> bool {
        matches!(
            self,
            Self::Key(KeyEvent {
                kind: KeyEventKind::Repeat,
                ..
            })
        )
    }

    /// Returns the key event if the event is a key event, otherwise `None`.
    #[inline]
    pub fn as_key_event(&self) -> Option<KeyEvent> {
        match self {
            Self::Key(event) => Some(*event),
            _ => None,
        }
    }

    /// Returns an Option containing the KeyEvent if the event is a key press event.
    #[inline]
    pub fn as_key_press_event(&self) -> Option<KeyEvent> {
        match self {
            Self::Key(event) if self.is_key_press() => Some(*event),
            _ => None,
        }
    }

    /// Returns an Option containing the `KeyEvent` if the event is a key release event.
    #[inline]
    pub fn as_key_release_event(&self) -> Option<KeyEvent> {
        match self {
            Self::Key(event) if self.is_key_release() => Some(*event),
            _ => None,
        }
    }

    /// Returns an Option containing the `KeyEvent` if the event is a key repeat event.
    #[inline]
    pub fn as_key_repeat_event(&self) -> Option<KeyEvent> {
        match self {
            Self::Key(event) if self.is_key_repeat() => Some(*event),
            _ => None,
        }
    }

    /// Returns the mouse event if the event is a mouse event, otherwise `None`.
    #[inline]
    pub fn as_mouse_event(&self) -> Option<MouseEvent> {
        match self {
            Self::Mouse(event) => Some(*event),
            _ => None,
        }
    }

    /// Returns the pasted string if the event is a paste event, otherwise `None`.
    #[inline]
    pub fn as_paste_event(&self) -> Option<&str> {
        match self {
            Self::Paste(paste) => Some(paste),
            _ => None,
        }
    }

    /// Returns the size as a tuple if the event is a terminal resize event, otherwise `None`.
    #[inline]
    pub fn as_terminal_resize_event(&self) -> Option<(u16, u16)> {
        match self {
            Self::TerminalResize(columns, rows) => Some((*columns, *rows)),
            _ => None,
        }
    }
}
