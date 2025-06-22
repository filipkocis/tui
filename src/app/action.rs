use std::collections::VecDeque;

use crossterm::event::KeyCode;

use crate::{Event, NodeId};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
/// This defines the actions that can be performed in the application.
/// Actions are typically emmited in event handlers, and are processed by the application's main
/// loop.
pub enum Action {
    /// Quit the application
    Quit,
    /// Emmit an event to the application.
    /// This may cause an `event -> action -> event` loop.
    EmmitEvent(Event),
    /// Emmit key inputs
    KeyInputs(Vec<KeyCode>),

    /// Focus the next node
    FocusNext,
    /// Focus the previous node
    FocusPrevious,
    /// Focus a specific node by its ID
    FocusNode(NodeId),
}

#[derive(Debug, Default)]
/// Queue of actions to be processed by the application.
/// Used inside the [app context](crate::AppContext).
pub struct Actions {
    /// Internal actions queue
    pub(crate) queue: VecDeque<Action>,
}

impl Actions {
    /// Create a new empty actions queue.
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    /// Adds an action to the queue.
    #[inline]
    pub fn emmit(&mut self, action: Action) {
        self.queue.push_back(action);
    }
}
