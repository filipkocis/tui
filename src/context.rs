use crossterm::event::Event;

use crate::{NodeId, WeakNodeHandle};

/// Used to store persistent context data for the application.
#[derive(Debug, Default)]
pub struct AppContext {
    /// Current hold position with it's target, from a mouse_down event.
    /// During drag, this field does not get changed automatically.
    pub hold: Option<(u16, u16, NodeId)>,

    /// Current node with focus. Set on mouse down event, before the event is dispatched. 
    /// Should be changed manually to implement more complex focus logic.    
    pub focus: Option<(NodeId, WeakNodeHandle)>,

    pub hover: Option<WeakNodeHandle>,
}

/// Unlike [`AppContext`], this context is used to store temporary per-event per-node data. Passed
/// to event handlers.
#[derive(Debug)]
pub struct Context<'a> {
    /// Application context.
    pub app: &'a mut AppContext,
    /// Current target node ID.
    pub target_id: NodeId,
    /// Current event.
    pub event: Event,

    /// Current target node.
    pub target_weak: WeakNodeHandle, 
    /// Current node.
    pub self_weak: WeakNodeHandle,

    /// True if executing the target phase.
    pub is_target_phase: bool,
}

impl<'a> Context<'a> {
    pub fn new(app: &'a mut AppContext, target_id: NodeId, event: Event, target_weak: WeakNodeHandle, self_weak: WeakNodeHandle) -> Self {
        Self {
            app,
            target_id,
            event,
            target_weak,
            self_weak,
            is_target_phase: false,
        }
    }
}
