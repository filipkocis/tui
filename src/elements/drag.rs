use crossterm::event::{KeyModifiers, MouseEventKind};

use crate::{Action, Context, IntoEventHandler, Node};

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
    /// Area is defined as (min_x, max_x) and (min_y, max_y). Both are optional and separate.
    pub fn new(
        area_x: Option<(u16, u16)>,
        area_y: Option<(u16, u16)>,
        modifiers: KeyModifiers,
    ) -> Node {
        let mut node = Node::default();
        Self::apply(&mut node, area_x, area_y, modifiers);
        node
    }

    /// Applies the draggable behavior to the given `node`.
    /// The node will be draggable within the specified `area_x` and `area_y` if the `modifiers`
    /// are used during he left-click drag event.
    /// # Notes
    /// Areas are restrictive, meaning that if they are `None`, the node can be dragged anywhere.
    /// Applying multiple draggable areas to the same node will cause unexpected behavior.
    pub fn apply(
        node: &mut Node,
        area_x: Option<(u16, u16)>,
        area_y: Option<(u16, u16)>,
        modifiers: KeyModifiers,
    ) {
        let on_drag = move |c: &mut Context, drag_event: MouseDragEvent, node: &mut Node| {
            let mut result = OnDragResult::default();

            if drag_event.modifiers != modifiers {
                return result;
            }

            let (x, y) = drag_event.relative;
            if let Some(area) = area_x {
                if x < area.0 || x > area.1 {
                    return result;
                }
            }
            if let Some(area) = area_y {
                if y < area.0 || y > area.1 {
                    return result;
                }
            }

            c.app.emmit(Action::RecomputeNode(c.self_weak.clone()));

            node.style.offset = node
                .style
                .offset
                .add_without_variant_change(drag_event.delta);

            result.stop_propagation = true;
            result.update_hold_x = true;
            result.update_hold_y = true;
            result
        };
        node.add_handler(on_drag_handler(on_drag), true);
    }
}
