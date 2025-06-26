use std::collections::VecDeque;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    focus::{Navigation, cycle_focus_flat},
    node::utils::get_parent_while,
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

    /// Recompute fully a node and it's children, then render the full tree.
    /// # Note
    /// Recomputing will start from the first **non-auto** sized node, so it may recompute more than
    /// just the node itself.
    RecomputeNode(WeakNodeHandle),
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
            Action::RecomputeNode(node_weak) => {
                let not_auto_sized = |node: &Node| {
                    !node.style.size.width.is_auto() && !node.style.size.height.is_auto()
                };

                let Some(mut node) = node_weak.upgrade() else {
                    return Ok(());
                };

                let node_borrow = node.borrow();
                let should_get_parent = node_borrow.cache().style.size != node_borrow.style.size
                    || !not_auto_sized(&node_borrow);
                drop(node_borrow);

                // If the node is auto-sized or has changed size, we take a parent node
                if should_get_parent {
                    // SAFETY: `None` is not possible as `node_weak` is guaranteed to be valid
                    node = get_parent_while(&node_weak, not_auto_sized)
                        .expect("Parent should exist")
                        .1;
                }

                let mut node = node.borrow_mut();
                let cached_parent_position = node.cache().parent_position;
                let cached_parent_available_size = node.cache().parent_available_size;

                node.compute(cached_parent_position, cached_parent_available_size);
                self.should_render = true;
            }
        }

        Ok(())
    }
}
