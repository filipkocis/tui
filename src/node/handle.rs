use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use super::Node;

#[derive(Debug, Clone)]
/// Handle to a non-owning node, used for parent references
pub struct WeakNodeHandle(Weak<RefCell<Node>>);

impl WeakNodeHandle {
    /// Get the inner `Weak<RefCell<Node>>`, use with cauton.
    #[inline(always)]
    fn inner(&self) -> &Weak<RefCell<Node>> {
        &self.0
    }

    /// Upgrades to a strong reference
    #[inline]
    pub fn upgrade(&self) -> Option<Rc<RefCell<Node>>> {
        self.inner().upgrade()
    }
}

#[derive(Debug)]
/// Handle to a node
pub struct NodeHandle(Rc<RefCell<Node>>);

impl NodeHandle {
    /// Constructs a new handle from a node
    pub fn new(node: Node) -> Self {
        Self(Rc::new(RefCell::new(node)))
    }

    /// Constructs a weak handle. Use with caution.
    #[inline]
    pub fn weak(&self) -> WeakNodeHandle {
        WeakNodeHandle(Rc::downgrade(self.inner()))
    }

    /// Returns the inner `Rc<RefCell<Node>>`, use with caution.
    #[inline(always)]
    pub fn inner(&self) -> &Rc<RefCell<Node>> {
        &self.0
    }

    /// Immutably borrows the node
    #[inline]
    pub fn borrow(&self) -> std::cell::Ref<Node> {
        self.inner().borrow()
    }

    /// Mutably borrows the node
    #[inline]
    pub fn borrow_mut(&self) -> std::cell::RefMut<Node> {
        self.inner().borrow_mut()
    }

    /// Adds a child node to the current node.
    pub fn add_child_node(&self, mut child: Node) {
        child.parent = Some(WeakNodeHandle(Rc::downgrade(self.inner())));
        let child = NodeHandle::new(child);

        self.borrow_mut().children.push(child);
    }

    /// Adds a child node handle to the current node.
    pub fn add_child(&self, child: NodeHandle) {
        child.borrow_mut().parent = Some(WeakNodeHandle(Rc::downgrade(self.inner())));
        self.borrow_mut().children.push(child);
    }
}
