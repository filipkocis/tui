use crossterm::event::{Event, KeyModifiers, MouseButton, MouseEventKind};

use crate::{IntoEventHandler, Node};

pub struct Button;

pub struct MouseClickEvent {
    pub button: MouseButton,
    pub relative: (u16, u16),
    pub modifiers: KeyModifiers,
}

pub type MouseClickHandler = Box<dyn FnMut(MouseClickEvent, &mut Node) -> bool>;

/// Generates an event handler for a mouse click event.
pub fn on_click_handler(mut on_click: impl FnMut(MouseClickEvent, &mut Node) -> bool + 'static) -> impl IntoEventHandler {
    move |event: &Event, node: &mut Node| {
        if let Event::Mouse(mouse_event) = event {
            let button = match mouse_event.kind {
                MouseEventKind::Down(button) => button,
                _ => return false,
            };

            let x = mouse_event.column as i16;
            let y = mouse_event.row as i16;

            if node.hit_test(x, y) {
                let click_event = MouseClickEvent { 
                    button,
                    relative: node.relative_position(x, y),
                    modifiers: mouse_event.modifiers,
                };

                return on_click(click_event, node)
            } 
        }
        false
    }
}

impl Button {
    /// Constructs a new buttom [`Node`](Node)
    pub fn new(label: String, on_click: Option<MouseClickHandler>) -> Node {
        let mut node = Node::default();
        let len = label.len() as u16;

        node.content = label;
        node.style.size = (len, 1);
        node.style.grow = true; 

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
