use crossterm::event::{Event, MouseEventKind};

use crate::{Context, Node, SizeValue};

pub struct Resizable;

impl Resizable {
    pub fn new(allow_x_resize: bool, allow_y_resize: bool) -> Node {
        let mut node = Node::default();

        let handler = move |ctx: &mut Context, event: &Event, node: &mut Node| {
            if !allow_x_resize && !allow_y_resize {
                return false;
            }

            let Some(mut hold) = ctx.hold else {
                return false;
            };

            let Some(mouse_event) = event.as_mouse_event() else {
                return false;
            };

            match mouse_event.kind {
                MouseEventKind::Drag(b) if b.is_left() => {}
                _ => return false,
            };

            let start_x = hold.0 as i16;
            let start_y = hold.1 as i16;
            let end_x = mouse_event.column;
            let end_y = mouse_event.row;

            let diff_x = end_x as i16 - start_x;
            let diff_y = end_y as i16 - start_y;

            let relative = node.relative_position(start_x, start_y);
            let total_width = node.style.total_width();
            let total_height = node.style.total_height();

            let drag_x = relative.0 + 1 == total_width;
            let drag_y = relative.1 + 1 == total_height;

            if !drag_x && !drag_y {
                return false;
            }

            if drag_x && allow_x_resize {
                let new_width = (total_width as i16 + diff_x).max(0) as u16;
                node.style.size.width = SizeValue::cells(new_width);
                hold.0 = end_x;
            }

            if drag_y && allow_y_resize {
                let new_height = (total_height as i16 + diff_y).max(0) as u16;
                node.style.size.height = SizeValue::cells(new_height);
                hold.1 = end_y;
            }

            ctx.hold = Some(hold);
            true
        };
        node.add_handler(handler, true);

        node
    }
}
