use std::{cell::RefCell, collections::VecDeque};

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

    /// Recompute a node and it's children, then re-render the full tree using a minimal viewport.
    /// # Note
    /// Recomputing starts with the first node (parent) that is not auto-sized, and does not have
    /// changed fields which might affect the layout (e.g. size, offset).
    RecomputeNode(WeakNodeHandle),

    /// Remove a Node from the root tree and recompute its parent
    RemoveNode(NodeId),
}

impl Action {
    /// Debug format but in a descriptive way, different from classic debug format
    pub fn descriptive_format(&self) -> String {
        fn node_id(weak: &WeakNodeHandle) -> String {
            weak.upgrade()
                .map(|n| n.try_borrow().map(|b| format!("{:?}", b.id())).ok())
                .flatten()
                .unwrap_or("NodeId(invalid)".into())
        }

        fn map_inputs(inputs: &[(KeyCode, KeyModifiers)]) -> Vec<String> {
            inputs
                .into_iter()
                .map(|(k, m)| format!("{m}+{k}"))
                .collect()
        }

        match self {
            Self::Quit => "Quit".into(),
            Self::EmmitEvent(e) => format!("EmmitEvent({e:?})"),
            Self::KeyInputs(k) => format!("KeyInputs({:?})", map_inputs(k)),
            Self::FocusNext => "FocusNext".into(),
            Self::FocusPrevious => "FocusPrevious".into(),
            Self::FocusNode(n) => format!("FocusNode({})", node_id(n)),
            Self::RecomputeNode(n) => format!("RecomputeNode({})", node_id(n)),
            Self::RemoveNode(id) => format!("RemoveNode({:?})", id),
        }
    }
}

#[derive(Debug, Default)]
/// Queue of actions to be processed by the application.
/// Used inside the [app context](crate::AppContext).
pub struct Actions {
    /// Internal actions queue
    queue: RefCell<VecDeque<Action>>,
}

impl Actions {
    /// Create a new empty actions queue.
    pub fn new() -> Self {
        Self {
            queue: RefCell::default(),
        }
    }

    /// Adds an action to the queue.
    #[inline]
    pub fn emmit(&self, action: Action) {
        self.queue.borrow_mut().push_back(action);
    }

    /// Drain the internal queue
    #[inline]
    fn drain(&self) -> Vec<Action> {
        self.queue.borrow_mut().drain(..).collect()
    }

    /// `true` if there are no actions in the internal queue
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.queue.borrow().is_empty()
    }
}

/// Implementing structs can handle actions in their context.
pub trait ActionHandling {
    /// Handles an action in the context of the application (the implementing struct).
    fn handle_action(&mut self, action: Action) -> std::io::Result<()>;
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
            Action::RecomputeNode(node_weak) => self.handle_recompute_node_action(node_weak),
            Action::RemoveNode(id) => {
                if let Some(parent) = self.remove_node(id).map(|(parent, _)| parent) {
                    self.handle_recompute_node_action(parent);
                }
            }
        }

        Ok(())
    }
}

impl App {
    /// Handles all actions in the queue.
    pub fn handle_actions(&mut self) -> std::io::Result<()> {
        let mut recomputed = Vec::<WeakNodeHandle>::new();

        for action in self.context.actions.drain() {
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

    /// Handle [Action::RecomputeNode]
    fn handle_recompute_node_action(&mut self, node_weak: WeakNodeHandle) {
        // True if the node is auto-sized, has changed size, or changed offset type.
        // Always false if the node is and was absolute, since that does not affect the
        // parent.
        let get_parent_predicate = |node: &Node| {
            let auto_sized = node.style.size.width.is_auto() || node.style.size.height.is_auto();
            let changed_size = node.cache().style.size != node.style.size;
            let changed_offset_type = !node.cache().style.offset.type_eq(node.style.offset);

            let was_absolute = node.cache().style.offset.is_absolute();
            let is_absolute = node.style.offset.is_absolute();

            if was_absolute && is_absolute {
                // If the node was and is absolute, don't get the parent
                return false;
            }

            // If the node is auto-sized, has changed size, or changed offset type
            auto_sized || changed_size || changed_offset_type
        };

        let Some((_, node)) = get_parent_while(&node_weak, get_parent_predicate) else {
            return;
        };

        let mut node = node.borrow_mut();
        let cached_parent_position = node.cache().parent_position;
        let cached_parent_available_size = node.cache().parent_available_size;
        let cached_position = node.cache().canvas_position;
        let cached_size = node.cache().style.total_size();
        let mut cached_viewport = node.cache().viewport;

        // Compute the tree starting from `node`
        node.compute(cached_parent_position, cached_parent_available_size);

        // If the node is absolute, we need to adjust the viewport in case it has moved
        // If it's relative, we cap the new viewport to the parent's content size.
        {
            let (x, y) = node.absolute_position();
            let (cache_x, cache_y) = cached_position;
            let (screen_x, screen_y) = self.context.screen_size;

            // Grab the smallest `min` position for the viewport
            cached_viewport.min.0 = (x.min(cache_x).max(0) as u16).min(screen_x);
            cached_viewport.min.1 = (y.min(cache_y).max(0) as u16).min(screen_y);

            // Compute new and old max positions (pos + size)
            let size = node.style.total_size();
            let new_max = (
                (x + size.0 as i16).max(0) as u16,
                (y + size.1 as i16).max(0) as u16,
            );
            let old_max = (
                (cache_x + cached_size.0 as i16).max(0) as u16,
                (cache_y + cached_size.1 as i16).max(0) as u16,
            );

            // Grab the largest `max` span for the viewport
            cached_viewport.max.0 = new_max.0.max(old_max.0).min(screen_x);
            cached_viewport.max.1 = new_max.1.max(old_max.1).min(screen_y);

            // Cap the viewport to the parent's available content size
            if node.style.offset.is_translate() {
                let min = cached_parent_position.tuple();
                let max = (
                    min.0 + cached_parent_available_size.tuple().0 as i16,
                    min.1 + cached_parent_available_size.tuple().1 as i16,
                );
                cached_viewport.min.0 = cached_viewport.min.0.max(min.0.max(0) as u16);
                cached_viewport.min.1 = cached_viewport.min.1.max(min.1.max(0) as u16);
                cached_viewport.max.0 = cached_viewport.max.0.min(max.0.max(0) as u16);
                cached_viewport.max.1 = cached_viewport.max.1.min(max.1.max(0) as u16);
            }
        }

        // Render the tree using the minimal viewport or the cached viewport which is the
        // parent's content size.
        drop(node);
        self.root
            .borrow()
            .render_to(cached_viewport, &mut self.canvas, &mut self.hitmap);
        self.should_draw = true;
    }
}
