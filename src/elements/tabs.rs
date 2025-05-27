use crossterm::style::Color;

use crate::{
    text::{CodeSpan, Text},
    Code, Node, NodeHandle, Offset, Padding, Size, SizeValue,
};

use super::Button;

pub struct Tabs;

impl Tabs {
    const COLOR: Color = Color::Rgb {
        r: 100,
        g: 20,
        b: 220,
    };

    pub fn new<V: 'static>(
        values: Vec<(String, V)>,
        default: usize,
        on_select: impl FnMut(&str, &V) + Clone + 'static,
    ) -> NodeHandle {
        let values_lengths = values
            .iter()
            .enumerate()
            .map(|(i, (label, _))| (i, label.len()))
            .collect::<Vec<_>>();
        let get_content = move |default: usize| -> (Text, u16) {
            // TODO: use Border options struct when implemented
            let to_right = '╰';
            let to_left = '╯';
            let connector = '┴';
            let straight = "─";

            let mut line = String::new();

            for (i, label_len) in &values_lengths {
                if *i == default {
                    line.push(to_left);
                    line.push_str(&" ".repeat(label_len + 2));
                    line.push(to_right);
                } else {
                    line.push(connector);
                    line.push_str(&straight.repeat(label_len + 2));
                    line.push(connector);
                }
            }

            let mut text = Text::plain(&line);
            let len = text.get_processed_size().0;

            text.set_style(vec![CodeSpan::new(
                Code::Foreground(Self::COLOR),
                0,
                len as usize,
            )]);
            (text, len)
        };

        let mut root = Node::default();
        root.style.size = Size::new(SizeValue::percent(100), SizeValue::cells(3));
        let mut bottom_line = Node::default();
        let mut tabs = Node::default();

        let (text, size) = get_content(default);
        bottom_line.text = text;
        bottom_line.style.offset = Offset::Translate(0, -1);
        bottom_line.style.size = Size::new(SizeValue::cells(size), SizeValue::cells(1));
        let bottom_line = bottom_line.into_handle();
        let bottom_line_weak = bottom_line.weak();

        tabs.style.size = Size::new(SizeValue::percent(100), SizeValue::cells(2));
        tabs.style.flex_row = true;
        tabs.style.border = (false, true, false, false, Some(Self::COLOR));
        let tabs = tabs.into_handle();

        for (i, (label, value)) in values.into_iter().enumerate() {
            let label_clone = label.clone();

            let mut on_select = on_select.clone();
            let bottom_line_weak = bottom_line_weak.clone();
            let get_content = get_content.clone();

            let on_click = Box::new(move |_, _: &mut _| {
                if let Some(line) = bottom_line_weak.upgrade() {
                    let mut line = line.borrow_mut();
                    let (text, size) = get_content(i);
                    line.text = text;
                    line.style.size.width = SizeValue::cells(size);
                }
                on_select(&label, &value);
                true
            });

            let mut button = Button::new(&label_clone, Some(on_click));
            button.style.padding = Padding::new(0, 1);
            button.style.border = (true, false, true, true, Some(Self::COLOR));

            tabs.add_child_node(button);
        }

        let root = root.into_handle();
        root.add_child(tabs);
        root.add_child(bottom_line);
        root
    }
}
