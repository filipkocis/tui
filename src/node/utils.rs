use std::{cell::RefCell, rc::Rc};

use crate::{Node, WeakNodeHandle};

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
