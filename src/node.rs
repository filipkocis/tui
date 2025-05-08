use std::{
    cell::RefCell,
    io::{stdout, Write},
    rc::Rc,
};

use crate::{offset::Offset, Canvas, Style, Viewport};

#[derive(Debug, Default)]
pub struct Node {
    pub id: String,
    pub class: String,
    pub style: Style,
    pub content: String,
    pub parent: Option<Rc<RefCell<Node>>>,
    pub children: Vec<Rc<RefCell<Node>>>,
    pub focus: bool,

    canvas: Canvas,
}

impl Node {
    /// Max possible width of the node
    pub fn max_width(&self) -> u16 {
        self.style.size.0
            + self.style.padding.2
            + self.style.padding.3
            + self.style.border.2 as u16
            + self.style.border.3 as u16
    }

    /// Max possible height of the node
    pub fn max_height(&self) -> u16 {
        self.style.size.1
            + self.style.padding.0
            + self.style.padding.1
            + self.style.border.0 as u16
            + self.style.border.1 as u16
    }

    pub fn calculate_canvas(&mut self, parent_position: Offset) {
        let position = parent_position.add(self.style.offset);
        let content_position = position.add_tuple((
            self.style.padding.2 as i16 + self.style.border.2 as i16,
            self.style.padding.0 as i16 + self.style.border.0 as i16,
        ));

        let mut canvas = Canvas {
            position: position.tuple(),
            buffer: vec![],
        };

        let mut extra_offset = (0, 0);
        let mut include_gap = false;
        for (i, child) in self.children.iter().enumerate() {
            let mut child = child.borrow_mut();
            child.calculate_canvas(content_position.add_tuple(extra_offset));

            if child.style.offset.is_absolute() {
                continue;
            }

            if self.style.flex_row {
                extra_offset.0 += child.canvas.width() as i16 + self.style.gap.0 as i16;
            } else {
                extra_offset.1 += child.canvas.height() as i16 + self.style.gap.1 as i16;
            }

            canvas.extend_child(
                &child.canvas,
                &self.style,
                include_gap,
                self.style.flex_row && i == 0,
            );

            include_gap = true;
        }
        canvas.add_text(&self.content, self.style.size);
        canvas.normalize(&self.style);

        canvas.add_padding(self.style.padding);
        canvas.add_fg(self.style.fg);
        canvas.add_bg(self.style.bg);
        canvas.add_border(self.style.border);

        self.canvas = canvas;
    }

    pub fn render(&self, mut viewport: Viewport) {
        viewport.min = (
            self.canvas.position.0.max(0) as u16,
            self.canvas.position.1.max(0) as u16,
        );

        let max = (
            (self.canvas.position.0 + self.max_width() as i16).max(0) as u16,
            (self.canvas.position.1 + self.max_height() as i16).max(0) as u16,
        );

        let abs_max = if self.style.offset.is_absolute() {
            viewport.screen
        } else {
            viewport.max
        };

        viewport.max = (max.0.min(abs_max.0), max.1.min(abs_max.1));

        self.canvas.render(&viewport);

        let overflow = (
            max.0.saturating_sub(viewport.screen.0),
            max.1.saturating_sub(viewport.screen.1),
        );

        viewport.max.0 -=
            (self.style.padding.3 + self.style.border.3 as u16).saturating_sub(overflow.0);
        viewport.max.1 -=
            (self.style.padding.1 + self.style.border.1 as u16).saturating_sub(overflow.1);

        for child in &self.children {
            let child = child.borrow();
            child.render(viewport);
        }

        stdout().flush().unwrap();
    }
}
