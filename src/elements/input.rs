use crossterm::event::KeyCode;

use crate::{
    text::{BufferLine, Text},
    Context, Node,
};

/// A simple text input element that allows for multi-line input.
/// The struct itself is used inside the [node's](Node) event handler.
pub struct Input {
    placeholder: String,
    visible_placeholder: bool,
    cursor: (usize, usize),
}

impl Input {
    pub fn new(placeholder: &str) -> Node {
        let mut root = Node::default();
        root.text = placeholder.into();
        root.text.cursor = Some((0, 0));

        let mut input = Self {
            placeholder: placeholder.to_string(),
            visible_placeholder: true,
            cursor: (0, 0),
        };

        let handler = move |c: &mut Context, node: &mut Node| {
            if let Some(paste) = c.event.as_paste_event() {
                // TODO implement auto splitting of lines in [`TextInput`] or [`Text`]
                input.add_string(paste, node);
                input.process_text(node);
            }

            let Some(key_event) = c.event.as_key_event() else {
                return false;
            };

            if key_event.code == KeyCode::Enter {
                input.add_new_line(node);
                input.process_text(node);
            }

            if key_event.code == KeyCode::Backspace {
                input.remove_char(node);
                input.process_text(node);
            }

            let Some(char) = key_event.code.as_char() else {
                return false;
            };

            if char.is_control() || char.is_ascii_control() {
                return false; // Ignore control characters
            }

            if char == '\n' {
                input.add_new_line(node);
            } else {
                input.add_char(char, node);
            }

            input.process_text(node);
            true
        };

        root.add_handler(handler, true);
        root
    }

    fn add_new_line(&mut self, node: &mut Node) {
        self.cursor.0 = 0;
        self.cursor.1 += 1;
        if self.visible_placeholder {
            node.text = "\n".into();
            self.visible_placeholder = false;
        } else {
            node.text.input.insert(self.cursor.1, BufferLine::default());
        }
    }

    fn add_char(&mut self, char: char, node: &mut Node) {
        if self.visible_placeholder {
            node.text = "".into();
            self.visible_placeholder = false;
        }

        let line = &mut node.text.input[self.cursor.1];
        let index = line
            .grapheme_to_byte_index(self.cursor.0)
            .unwrap_or(line.content().len());
        line.insert(index, char);

        self.cursor.0 += 1;
    }

    fn add_string(&mut self, text: &str, node: &mut Node) {
        if self.visible_placeholder {
            node.text = text.into();
            let lines_len = node.text.input.len();
            let final_line_len = node.text.input.last().map_or(0, |l| l.count());
            self.cursor = (final_line_len, lines_len.saturating_sub(1));
            self.visible_placeholder = false;
            return;
        }

        let text: Text = text.into();
        let new_lines = text.input;
        let lines = &mut node.text.input;

        if new_lines.len() == 1 {
            let line = &mut lines[self.cursor.1];
            let index = line
                .grapheme_to_byte_index(self.cursor.0)
                .unwrap_or(line.content().len());
            line.insert_str(index, new_lines[0].content());
            self.cursor.0 += new_lines[0].count();
        } else {
            let old_line = lines.remove(self.cursor.1);
            let mid_index = old_line
                .grapheme_to_byte_index(self.cursor.0)
                .unwrap_or(old_line.content().len());
            let (left_part, right_part) = old_line.content().split_at(mid_index);

            self.cursor.0 = new_lines.last().map(|l| l.count()).unwrap_or(0);

            let original_y = self.cursor.1;
            for line in new_lines {
                lines.insert(self.cursor.1, line);
                self.cursor.1 += 1;
            }
            self.cursor.1 -= 1; // Adjust to the last inserted line

            if !left_part.is_empty() {
                lines[original_y].insert_str(0, left_part);
            }

            if !right_part.is_empty() {
                lines[self.cursor.1].push_str(right_part);
            }
        }
    }

    fn remove_char(&mut self, node: &mut Node) {
        let lines = &mut node.text.input;
        if self.cursor.0 > 0 {
            self.cursor.0 -= 1;
            lines[self.cursor.1].remove(self.cursor.0);
        } else if self.cursor.1 > 0 {
            let prev_line = lines.remove(self.cursor.1);
            self.cursor.1 -= 1;
            self.cursor.0 = lines[self.cursor.1].count();
            lines[self.cursor.1].push_str(prev_line.content());
        }

        if lines.len() == 1 && lines[0].content().is_empty() {
            self.visible_placeholder = true;
        }
    }

    fn process_text(&mut self, node: &mut Node) {
        if self.visible_placeholder {
            node.text = self.placeholder.clone().into();
            self.cursor = (0, 0);
        } else {
            node.text.prepare_text(u16::MAX);
        }

        node.text.cursor = Some((self.cursor.0 as u16, self.cursor.1 as u16));
    }
}
