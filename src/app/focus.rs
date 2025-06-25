use std::{cell::RefCell, rc::Rc};

use crate::*;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
/// Represents the direction of navigation (e.g., focus cycling).
pub enum Navigation {
    #[default]
    Next,
    Previous,
}

/// Returns the first parent node and its weak handle that does not match the predicate, or the top
/// of the tree if no such parent is found. Returns `None` if the weak handle cannot be upgraded.
///
/// # Safety
/// If any nodes are borrowed mutably, this will not work correctly.
pub fn get_parent_while<P>(
    weak: &WeakNodeHandle,
    predicate: P,
) -> Option<(WeakNodeHandle, Rc<RefCell<Node>>)>
where
    P: Fn(&Node) -> bool,
{
    let node = weak.upgrade()?;
    let node_borrow = node.try_borrow().ok()?;

    let mut parent = node_borrow
        .parent
        .as_ref()
        .and_then(|p| p.upgrade().map(|n| (p.clone(), n)));

    while let Some((p_weak, p)) = parent.take() {
        let p_clone = p.clone();
        let p_borrow = p_clone.try_borrow().ok()?;

        // if the predicate is not met, we stop here
        if !predicate(&p_borrow) {
            parent = Some((p_weak, p));
            break;
        }

        match p_borrow
            .parent
            .as_ref()
            .and_then(|p| p.upgrade().map(|n| (p.clone(), n)))
        {
            // continue up the tree
            Some(new_parent) => parent = Some(new_parent),
            // we reached the top of the tree, or an invalid weak parent
            None => {
                parent = Some((p_weak, p));
                break;
            }
        }
    }

    drop(node_borrow);
    Some(parent.unwrap_or((weak.clone(), node)))
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
