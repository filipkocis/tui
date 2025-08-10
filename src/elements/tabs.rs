use crossterm::style::Color;

use crate::{
    Border, Code, Context, Node, NodeHandle, Offset, Padding, Size, SizeValue,
    border::BorderStyle,
    text::{StyleSpan, Text},
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
        on_select: impl FnMut(&Context, &str, &V) + Clone + 'static,
    ) -> NodeHandle {
        let values_lengths = values
            .iter()
            .enumerate()
            .map(|(i, (label, _))| (i, label.len()))
            .collect::<Vec<_>>();
        let build_line_text = move |default: usize, style: &BorderStyle| -> (Text, u16) {
            // TODO: add connectors to border style
            let connector = '┴';

            let mut line = String::new();

            for (i, label_len) in &values_lengths {
                if *i == default {
                    line.push_str(&style.bottom_right);
                    line.push_str(&" ".repeat(label_len + 2));
                    line.push_str(&style.bottom_left);
                } else {
                    line.push(connector);
                    line.push_str(&style.bottom.repeat(label_len + 2));
                    line.push(connector);
                }
            }

            let mut text = Text::plain(&line);
            let len = text.get_visual_size().0;

            text.add_styles(vec![StyleSpan::new(
                Code::Foreground(Self::COLOR),
                0,
                0,
                len as usize,
            )]);

            (text, len)
        };

        let mut root = Node::default();
        root.style.size = Size::new(SizeValue::percent(100), SizeValue::cells(3));
        let mut bottom_line = Node::default();
        let mut tabs = Node::default();

        let (text, size) = build_line_text(default, bottom_line.style.border.style);
        bottom_line.text = text;
        bottom_line.style.offset = Offset::Translate(0, -1);
        bottom_line.style.size = Size::new(SizeValue::cells(size), SizeValue::cells(1));
        let bottom_line = bottom_line.into_handle();
        let bottom_line_weak = bottom_line.weak();

        tabs.style.size = Size::new(SizeValue::percent(100), SizeValue::cells(2));
        tabs.style.flex_row = true;
        tabs.style.border = Border::none()
            .with_bottom(true)
            .with_color(Some(Self::COLOR));
        let tabs = tabs.into_handle();

        for (i, (label, value)) in values.into_iter().enumerate() {
            let label_clone = label.clone();

            let mut on_select = on_select.clone();
            let bottom_line_weak = bottom_line_weak.clone();
            let build_line_text = build_line_text.clone();

            let on_click = Box::new(move |ctx: &mut Context, _, _: &mut _| {
                if let Some(line) = bottom_line_weak.upgrade() {
                    let mut line = line.borrow_mut();
                    let (text, size) = build_line_text(i, line.style.border.style);
                    line.text = text;
                    line.style.size.width = SizeValue::cells(size);
                }
                on_select(ctx, &label, &value);
                true
            });

            let mut button = Button::new(&label_clone, Some(on_click));
            button.style.padding = Padding::new(0, 1);
            button.style.border = Border::all()
                .with_bottom(false)
                .with_color(Some(Self::COLOR));

            tabs.add_child_node(button);
        }

        let root = root.into_handle();
        root.add_child(tabs);
        root.add_child(bottom_line);
        root
    }
}
