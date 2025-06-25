use std::{cell::RefCell, rc::Rc};

use crate::{node::utils::get_parent_while, *};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
/// Represents the direction of navigation (e.g., focus cycling).
pub enum Navigation {
    #[default]
    Next,
    Previous,
}

/// Cycles the focus based on the current `weak_focus`. The cycle will be contained within the `root_id`
/// node if provided, and will wrap around if enabled. `Navigation` determines the direction of the
/// cycle (next or previous.)
///
/// # Safety
/// If any nodes are borrowed mutably, this will not work correctly.
///
/// # Returns
/// Returns the new focused node if any focus change occured.
pub fn cycle_focus_flat(
    weak_focus: WeakNodeHandle,
    root_id: Option<NodeId>,
    navigation: Navigation,
    wrapping: bool,
) -> Option<(NodeId, WeakNodeHandle)> {
    let focus_id = {
        let focus = weak_focus.upgrade()?;
        let focus = focus.try_borrow().ok()?;
        focus.id()
    };

    let not_root = |node: &Node| Some(node.id()) != root_id;
    let (weak_container, container) = match get_parent_while(&weak_focus, not_root) {
        Some(container) => container,
        None => return None,
    };

    let mut nodes: Vec<(NodeId, WeakNodeHandle)> = vec![(container.borrow().id(), weak_container)];

    /// Recursively collects all nodes in the tree starting from the `parent` node.
    fn collect_nodes(nodes: &mut Vec<(NodeId, WeakNodeHandle)>, parent: &Rc<RefCell<Node>>) {
        for child in &parent.borrow().children {
            let weak = child.weak();
            nodes.push((child.borrow().id(), weak.clone()));
            collect_nodes(nodes, child.inner());
        }
    }
    collect_nodes(&mut nodes, &container);

    if nodes.is_empty() {
        return None; // No nodes to cycle through
    }

    let focus_index = nodes
        .iter()
        .position(|(id, _)| *id == focus_id)
        .expect("Focus node should be in the list");

    let next_index = match navigation {
        Navigation::Next => {
            if focus_index + 1 < nodes.len() {
                focus_index + 1
            } else if wrapping {
                0
            } else {
                return None;
            }
        }
        Navigation::Previous => {
            if focus_index > 0 {
                focus_index - 1
            } else if wrapping {
                nodes.len() - 1
            } else {
                return None;
            }
        }
    };

    let (next_id, next_weak) = nodes[next_index].clone();
    if next_id == focus_id {
        return None; // No change in focus
    }

    Some((next_id, next_weak))
}
