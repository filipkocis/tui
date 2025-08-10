use crossterm::event::{KeyModifiers, MouseEventKind};

use crate::{Action, Context, IntoEventHandler, Node, PartialRect, WeakNodeHandle};

#[derive(Debug, Clone)]
/// Synthetic event for left-click mouse drag events.
pub struct MouseDragEvent {
    /// Delta from between `drag start` and `drag end` positions.
    pub delta: (i16, i16),
    /// Drag start position relative to the node.
    pub relative: (u16, u16),
    /// Drage end position, in absolute coordinates.
    pub absolute: (u16, u16),
    /// Key modifiers at the time of the drag event.
    pub modifiers: KeyModifiers,
}

#[derive(Debug, Default, Clone, Copy)]
/// Return value of the `on_drag` event handler callback.
pub struct OnDragResult {
    /// Indicates if event handling should stop.
    pub stop_propagation: bool,
    /// If true, `context.hold` X value will be updated to `drag end` X value.
    pub update_hold_x: bool,
    /// If true, `context.hold` Y value will be updated to `drag end` Y value.
    pub update_hold_y: bool,
}

/// Generates an event handler for a synthetic left-click mouse drag event.
pub fn on_drag_handler(
    mut on_drag: impl FnMut(&mut Context, MouseDragEvent, &mut Node) -> OnDragResult + 'static,
) -> impl IntoEventHandler {
    move |ctx: &mut Context, node: &mut Node| {
        // Has hold position
        let Some(mut hold) = ctx.app.hold else {
            return false;
        };

        // Is grabbed by this node
        if hold.2 != node.id() {
            return false;
        }

        // Is mouse event
        let Some(mouse_event) = ctx.event.as_mouse_event() else {
            return false;
        };

        // Is left-click mouse drag event
        match mouse_event.kind {
            MouseEventKind::Drag(button) if button.is_left() => {}
            _ => return false,
        };

        let start = (hold.0 as i16, hold.1 as i16);
        let end = (mouse_event.column, mouse_event.row);
        let delta = (end.0 as i16 - start.0, end.1 as i16 - start.1);

        let drag_event = MouseDragEvent {
            delta,
            relative: node.relative_position(start.0, start.1),
            absolute: (end.0, end.1),
            modifiers: mouse_event.modifiers,
        };

        let result = on_drag(ctx, drag_event, node);

        if result.update_hold_x {
            hold.0 = end.0;
        }

        if result.update_hold_y {
            hold.1 = end.1;
        }

        ctx.app.hold = Some(hold);

        result.stop_propagation
    }
}

pub struct Draggable;

impl Draggable {
    /// Constructs a new [`Node`] that can be dragged around if `modifiers` are used during the
    /// left-click drag event, and when it's within node's `area`.
    /// Drag area is defined by a `PartialRect` which can be used to restrict the drag area.
    pub fn new(drag_area: PartialRect, modifiers: KeyModifiers) -> Node {
        let mut node = Node::default();
        Self::apply(&mut node, drag_area, modifiers, None);
        node
    }

    /// Applies the draggable behavior to the given `node`.
    /// The node is draggable if `LMB` is pressed and the `modifiers` match, and only if the mouse
    /// is within the designated `drag_area`.
    /// If `target` is provided, the drag event will modify the target node's position instead of
    /// the `node` itself.
    /// # Notes
    /// Areas are restrictive, meaning that if they are `None`, the node can be dragged anywhere.
    /// Applying multiple draggable areas to the same node will cause unexpected behavior.
    pub fn apply(
        node: &mut Node,
        drag_area: PartialRect,
        modifiers: KeyModifiers,
        target: Option<WeakNodeHandle>,
    ) {
        let on_drag = move |c: &mut Context, drag_event: MouseDragEvent, node: &mut Node| {
            let mut result = OnDragResult::default();

            if drag_event.modifiers != modifiers {
                return result;
            }

            let (x, y) = drag_event.relative;
            if !drag_area.contains(x, y) {
                return result;
            }

            // If a target is specified, modify the target node's position
            // otherwise modify the node's position directly.
            if let Some(target) = &target {
                let Some(target_node) = target.upgrade() else {
                    return result;
                };
                let Ok(mut target_node) = target_node.try_borrow_mut() else {
                    return result;
                };

                target_node.style.offset = target_node
                    .style
                    .offset
                    .add_without_variant_change(drag_event.delta);

                c.app.emmit(Action::RecomputeNode(target.clone()));
            } else {
                node.style.offset = node
                    .style
                    .offset
                    .add_without_variant_change(drag_event.delta);

                c.app.emmit(Action::RecomputeNode(c.self_weak.clone()));
            }

            result.stop_propagation = true;
            result.update_hold_x = true;
            result.update_hold_y = true;
            result
        };
        node.add_handler(on_drag_handler(on_drag), true);
    }
}
