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
