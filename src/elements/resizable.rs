use crate::{Context, Node, SizeValue};

use super::{MouseDragEvent, OnDragResult, on_drag_handler};

pub struct Resizable;

impl Resizable {
    pub fn new(allow_x_resize: bool, allow_y_resize: bool) -> Node {
        let mut node = Node::default();

        let on_drag = move |_: &mut Context, drag_event: MouseDragEvent, node: &mut Node| {
            let mut result = OnDragResult::default();

            if !allow_x_resize && !allow_y_resize {
                return result;
            }

            let total_width = node.style.total_width();
            let total_height = node.style.total_height();

            let drag_x = drag_event.relative.0 + 1 == total_width;
            let drag_y = drag_event.relative.1 + 1 == total_height;

            // Drag only on the node's edge
            if !drag_x && !drag_y {
                return result;
            }

            // Dont resize if dragging is happening at < than the node's offset,
            // relative is clamped to 0 and may cause false positives
            if drag_event.absolute.0 < node.style.offset.x().max(0) as u16
                || drag_event.absolute.1 < node.style.offset.y().max(0) as u16
            {
                return result;
            }

            if drag_x && allow_x_resize {
                let content_width = node.style.size.width.computed_size();
                let new_width = (content_width as i16 + drag_event.delta.0).max(0) as u16;
                node.style.size.width = SizeValue::cells(new_width);
                result.update_hold_x = true;
            }

            if drag_y && allow_y_resize {
                let content_height = node.style.size.height.computed_size();
                let new_height = (content_height as i16 + drag_event.delta.1).max(0) as u16;
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
