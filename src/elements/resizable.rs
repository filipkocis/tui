use crate::{Action, Context, Node, SizeValue};

use super::{MouseDragEvent, OnDragResult, on_drag_handler};

pub struct Resizable;

impl Resizable {
    /// Creates a new [Node] with resizable functionality in both x and y directions.
    pub fn new(allow_x_resize: bool, allow_y_resize: bool) -> Node {
        let mut node = Node::default();
        Self::apply(&mut node, allow_x_resize, allow_y_resize);
        node
    }

    /// Applies resizable functionality to the given [Node]. Adds a mouse drag handler that allows
    /// resizing the node at its edges.
    /// # Notes
    /// Applying this multiple times to the same node will cause unexpected behavior.
    pub fn apply(node: &mut Node, allow_x_resize: bool, allow_y_resize: bool) {
        let on_drag = move |ctx: &mut Context, drag_event: MouseDragEvent, node: &mut Node| {
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

            let content_width = node.style.size.width.computed_size();
            let new_width = content_width as i16 + drag_event.delta.0;
            if drag_x && allow_x_resize && new_width >= 0 {
                node.style.size.width = SizeValue::cells(new_width as u16);
                result.update_hold_x = true;
                result.stop_propagation = true;
            }

            let content_height = node.style.size.height.computed_size();
            let new_height = content_height as i16 + drag_event.delta.1;
            if drag_y && allow_y_resize && new_height >= 0 {
                node.style.size.height = SizeValue::cells(new_height as u16);
                result.update_hold_y = true;
                result.stop_propagation = true;
            }

            if result.stop_propagation {
                ctx.app.emmit(Action::RecomputeNode(ctx.self_weak.clone()));
            }

            result
        };
        node.add_handler(on_drag_handler(on_drag), true);
    }
}
