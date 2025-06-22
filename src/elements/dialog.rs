use crossterm::{event::KeyModifiers, style::Color};

use crate::{Node, Offset, Padding, Size, SizeValue, node::NodeHandle, text::Text};

use super::{Button, Draggable, button::MouseClickHandler};

pub struct Dialog;

impl Dialog {
    /// Returns a title text node
    fn title(title: &str) -> Node {
        let mut node = Node::default();
        node.text = Text::plain(title);
        node.style.bold = true;
        node
    }

    /// Returns a message text node
    fn message(message: &str) -> Node {
        let mut node = Node::default();
        node.text = Text::plain(message);
        node.style.max_size = Size::new(SizeValue::percent(100), SizeValue::auto());
        node
    }

    /// Returns a button node
    fn button(label: &str, on_click: Option<MouseClickHandler>, bg: Option<Color>) -> Node {
        let mut node = Button::new(label, on_click);
        node.style.bg = bg;
        node.style.padding = Padding::new(0, 2);
        node
    }

    /// Returns a container node for buttons
    fn buttons_container() -> NodeHandle {
        let mut buttons = Node::default();
        buttons.style.size = Size::new(SizeValue::auto(), SizeValue::cells(1));
        buttons.style.flex_row = true;
        buttons.style.gap = (1, 1);
        buttons.style.padding = Padding::top(1);

        buttons.into_handle()
    }

    /// Returns a container node for the dialog
    fn container(y: i16) -> NodeHandle {
        let mut container = Draggable::new(None, None, KeyModifiers::CONTROL);
        container.style.offset = Offset::Absolute(0, y);
        container.style.size = Size::new(SizeValue::cells(30), SizeValue::auto());
        container.style.gap = (1, 1);
        container.style.padding = Padding::new(1, 2);
        container.style.border = (
            true,
            true,
            true,
            true,
            Some(Color::Rgb {
                r: 138,
                g: 43,
                b: 226,
            }),
        );

        container.into_handle()
    }

    /// Constructs a new dialog [`Node`](Node) with a title, message, and two buttons.
    pub fn dialog(
        title: &str,
        message: &str,
        on_action: Option<MouseClickHandler>,
        action_label: Option<&str>,
        on_cancel: Option<MouseClickHandler>,
        cancel_label: Option<&str>,
    ) -> NodeHandle {
        let node = Self::container(0);
        if title.len() > 0 {
            let title = Self::title(title);
            node.add_child_node(title);
        }
        if message.len() > 0 {
            let message = Self::message(message);
            node.add_child_node(message);
        }

        let buttons = Self::buttons_container();
        let action = Self::button(
            action_label.unwrap_or("OK"),
            on_action,
            Some(Color::Rgb {
                r: 255,
                g: 0,
                b: 255,
            }),
        );
        let cancel = Self::button(
            cancel_label.unwrap_or("Cancel"),
            on_cancel,
            Some(Color::Rgb {
                r: 170,
                g: 170,
                b: 170,
            }),
        );
        buttons.add_child_node(action);
        buttons.add_child_node(cancel);
        node.add_child(buttons);

        node
    }

    /// Constructs a new alert [`Node`](Node) with a title, message, and one button.
    pub fn alert(
        title: &str,
        message: &str,
        on_action: Option<MouseClickHandler>,
        action_label: Option<&str>,
    ) -> NodeHandle {
        let node = Self::container(30);
        if title.len() > 0 {
            let title = Self::title(title);
            node.add_child_node(title);
        }
        if message.len() > 0 {
            let message = Self::message(message);
            node.add_child_node(message);
        }

        let buttons = Self::buttons_container();
        let action = Self::button(
            action_label.unwrap_or("OK"),
            on_action,
            Some(Color::Rgb {
                r: 138,
                g: 43,
                b: 226,
            }),
        );
        buttons.add_child_node(action);
        node.add_child(buttons);

        node
    }
}
