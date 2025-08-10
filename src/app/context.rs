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
    pub(crate) focus: Option<(NodeId, WeakNodeHandle)>,
    /// Current node under the mouse cursor. Set on mouse move event, before the event is
    /// dispatched.
    pub(crate) hover: Option<(NodeId, WeakNodeHandle)>,

    /// Current screen size
    pub(crate) screen_size: (u16, u16),
    /// Current mouse position (in screen coords). It will be `None` if the app never
    /// received a mouse event.
    pub(crate) mouse_pos: Option<(u16, u16)>,

    /// Actions queue for the application. Executed in the main loop.
    pub actions: Actions,
}

impl AppContext {
    /// Creates a new [`app`](crate::App) context with a default focused node `root` and a set
    /// `screen_size`
    pub fn new(root: &NodeHandle, screen_size: (u16, u16)) -> Self {
        Self {
            hold: None,
            focus: Some((root.borrow().id(), root.weak())),
            hover: None,
            screen_size,
            mouse_pos: None,
            actions: Actions::new(),
        }
    }

    /// Emmit an action
    #[inline]
    pub fn emmit(&self, action: Action) {
        self.actions.emmit(action);
    }

    /// Currently focused node.
    #[inline]
    pub fn focus(&self) -> &Option<(NodeId, WeakNodeHandle)> {
        &self.focus
    }

    /// Current node under the mouse cursor.
    #[inline]
    pub fn hover(&self) -> &Option<(NodeId, WeakNodeHandle)> {
        &self.hover
    }

    /// Current screen size.
    #[inline]
    pub fn screen_size(&self) -> (u16, u16) {
        self.screen_size
    }

    /// Current mouse positoin (in screen coords).
    #[inline]
    pub fn mouse_pos(&self) -> Option<(u16, u16)> {
        self.mouse_pos
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
