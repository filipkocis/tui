use crossterm::event::{KeyModifiers, MouseButton, MouseEventKind};

use crate::{Context, IntoEventHandler, Node, Size, text::Text};

pub struct Button;

pub struct MouseClickEvent {
    pub button: MouseButton,
    pub relative: (u16, u16),
    pub modifiers: KeyModifiers,
}

pub type MouseClickHandler = Box<dyn FnMut(&mut Context, MouseClickEvent, &mut Node) -> bool>;

/// Generates an event handler for a mouse click event.
pub fn on_click_handler(
    mut on_click: impl FnMut(&mut Context, MouseClickEvent, &mut Node) -> bool + 'static,
) -> impl IntoEventHandler {
    move |c: &mut Context, node: &mut Node| {
        let Some(mouse_event) = c.event.as_mouse_event() else {
            return false;
        };

        let button = match mouse_event.kind {
            MouseEventKind::Down(button) => button,
            _ => return false,
        };

        let x = mouse_event.column as i16;
        let y = mouse_event.row as i16;

        let click_event = MouseClickEvent {
            button,
            relative: node.relative_position(x, y),
            modifiers: mouse_event.modifiers,
        };

        on_click(c, click_event, node)
    }
}

impl Button {
    /// Constructs a new buttom [`Node`](Node)
    pub fn new(label: &str, on_click: Option<MouseClickHandler>) -> Node {
        let mut node = Node::default();
        node.text = Text::plain(label);
        let width = node.text.get_visual_size().0;
        node.style.size = Size::from_cells(width, 1);
        // node.style.grow = true;

        // node.add_handler(
        //     on_click_handler(|event, node| {
        //         node.set_focused(true);
        //         return false
        //     }),
        //     false
        // );

        if let Some(on_click) = on_click {
            node.add_handler(on_click_handler(on_click), false)
        }

        node
    }
}
