use std::{
    cell::{Ref, RefCell, RefMut},
    rc::{Rc, Weak},
};

use super::Node;

#[derive(Debug, Clone)]
/// Handle to a non-owning node, used for parent references
pub struct WeakNodeHandle(Weak<RefCell<Node>>);

impl WeakNodeHandle {
    /// Creates a new weak handle from the inner type
    #[inline(always)]
    pub(crate) fn new(inner: Weak<RefCell<Node>>) -> Self {
        Self(inner)
    }

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

    /// `true` if `self` and `other` point to the same weak node.
    #[inline]
    pub fn is_equal(&self, other: &Self) -> bool {
        Weak::ptr_eq(self.inner(), other.inner())
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

    /// `true` if `self` and `other` point to the same node.
    #[inline]
    pub fn is_equal(&self, other: &Self) -> bool {
        Rc::ptr_eq(self.inner(), other.inner())
    }

    /// Returns the inner `Rc<RefCell<Node>>`, use with caution.
    #[inline(always)]
    pub fn inner(&self) -> &Rc<RefCell<Node>> {
        &self.0
    }

    /// Immutably borrows the node
    #[inline]
    pub fn borrow(&self) -> Ref<'_, Node> {
        self.inner().borrow()
    }

    /// Mutably borrows the node
    #[inline]
    pub fn borrow_mut(&self) -> RefMut<'_, Node> {
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
