use crate::{Action, Event, NodeHandle, NodeId, WeakNodeHandle, action::Actions};

/// Used to store persistent context data for the application.
#[derive(Debug, Default)]
pub struct AppContext {
    /// Current hold position with it's target, from a mouse_down event.
    /// During drag, this field does not get changed automatically.
    pub hold: Option<(u16, u16, NodeId)>,
    /// Current node with focus. Set on mouse down event, before the event is dispatched.
    /// Should be changed manually to implement more complex focus logic.    
    /// Initially set to the root node.
    pub focus: Option<(NodeId, WeakNodeHandle)>,

    /// TODO: this one has no use yet
    pub hover: Option<WeakNodeHandle>,

    /// Screen size
    pub screen_size: (u16, u16),

    /// Actions queue for the application. Executed in the main loop.
    pub actions: Actions,
}

impl AppContext {
    pub fn new(root: &NodeHandle) -> Self {
        Self {
            hold: None,
            focus: Some((root.borrow().id(), root.weak())),
            hover: None,
            screen_size: (0, 0),
            actions: Actions::new(),
        }
    }

    /// Emmit an action
    #[inline]
    pub fn emmit(&mut self, action: Action) {
        self.actions.emmit(action);
    }
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
    pub fn new(
        app: &'a mut AppContext,
        target_id: NodeId,
        event: Event,
        target_weak: WeakNodeHandle,
        self_weak: WeakNodeHandle,
    ) -> Self {
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
