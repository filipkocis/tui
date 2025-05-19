use crate::{Context, Node, SizeValue};

use super::{on_drag_handler, MouseDragEvent, OnDragResult};

pub struct Resizable;

impl Resizable {
    pub fn new(allow_x_resize: bool, allow_y_resize: bool) -> Node {
        let mut node = Node::default();

        let on_drag = move |ctx: &mut Context, drag_event: MouseDragEvent, node: &mut Node| {
            let mut result = OnDragResult::default();

            if !allow_x_resize && !allow_y_resize {
                return result;
            }

            let total_width = node.style.total_width();
            let total_height = node.style.total_height();

            let drag_x = drag_event.relative.0 + 1 == total_width;
            let drag_y = drag_event.relative.1 + 1 == total_height;

            if !drag_x && !drag_y {
                return result;
            }

            if drag_x && allow_x_resize {
                let new_width = (total_width as i16 + drag_event.delta.0).max(0) as u16;
                node.style.size.width = SizeValue::cells(new_width);
                result.update_hold_x = true;
            }

            if drag_y && allow_y_resize {
                let new_height = (total_height as i16 + drag_event.delta.1).max(0) as u16;
                node.style.size.height = SizeValue::cells(new_height);
                result.update_hold_y = true;
            }

            result.stop_propagation = true;
            result
        };
        node.add_handler(on_drag_handler(on_drag), true);

        node
    }
}
