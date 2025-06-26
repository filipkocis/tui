use std::collections::VecDeque;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    focus::{Navigation, cycle_focus_flat},
    *,
};

#[derive(Debug, Clone)]
/// This defines the actions that can be performed in the application.
/// Actions are typically emmited in event handlers, and are processed by the application's main
/// loop.
pub enum Action {
    /// Quit the application
    Quit,
    /// Emmit an event to the application.
    /// This may cause an `event -> action -> event` loop.
    EmmitEvent(crossterm::event::Event),
    /// Emmit key inputs with modifiers. Each key will be processed as `press`ed and `release`d
    KeyInputs(Vec<(KeyCode, KeyModifiers)>),

    /// Focus the next node
    FocusNext,
    /// Focus the previous node
    FocusPrevious,
    /// Focus a specific node
    FocusNode(WeakNodeHandle),

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

/// Implementing structs can handle actions in their context.
pub trait ActionHandling {
    /// Handles an action in the context of the application (the implementing struct).
    fn handle_action(&mut self, action: Action) -> std::io::Result<()>;
}

impl App {
    /// Handles all actions in the queue.
    pub fn handle_actions(&mut self) -> std::io::Result<()> {
        let mut recomputed = Vec::<WeakNodeHandle>::new();

        for action in self.context.actions.queue.drain(..).collect::<Vec<_>>() {
            match &action {
                Action::RecomputeNode(weak) => {
                    if recomputed.iter().any(|r| r.is_equal(weak)) {
                        continue; // Already recomputed this node
                    }
                    recomputed.push(weak.clone());
                }
                _ => {}
            }

            self.handle_action(action)?
        }

        Ok(())
    }
}

impl ActionHandling for App {
    fn handle_action(&mut self, action: Action) -> std::io::Result<()> {
        match action {
            Action::Quit => self.should_quit = true,
            Action::EmmitEvent(event) => self.handle_crossterm_event(event)?,
            Action::KeyInputs(key_inputs) => {
                for (key, modifiers) in key_inputs {
                    let mut key_event = KeyEvent {
                        code: key,
                        modifiers,
                        kind: crossterm::event::KeyEventKind::Press,
                        state: crossterm::event::KeyEventState::NONE,
                    };
                    if self.should_quit(&key_event) {
                        return Ok(());
                    }

                    self.dispatch_key_event(key_event);
                    key_event.kind = crossterm::event::KeyEventKind::Release;
                    self.dispatch_key_event(key_event);
                }
            }
            Action::FocusNext => {
                if let Some((_, focus_weak)) = self.context.focus.clone() {
                    if let Some((new_focus_id, new_focus_weak)) =
                        cycle_focus_flat(focus_weak, None, Navigation::Next, true)
                    {
                        self.dispatch_node_focus_event(new_focus_id, new_focus_weak);
                    }
                }
            }
            Action::FocusPrevious => {
                if let Some((_, focus_weak)) = self.context.focus.clone() {
                    if let Some((new_focus_id, new_focus_weak)) =
                        cycle_focus_flat(focus_weak, None, Navigation::Previous, true)
                    {
                        self.dispatch_node_focus_event(new_focus_id, new_focus_weak);
                    }
                }
            }
            Action::FocusNode(node_weak) => {
                let Some(node) = node_weak.upgrade() else {
                    return Ok(());
                };

                let node_id = node.borrow().id();
                self.dispatch_node_focus_event(node_id, node_weak);
            }

        }

        Ok(())
    }
}
