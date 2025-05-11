use std::{cell::RefCell, rc::Rc};

use super::Node;

#[derive(Debug)]
/// Handle to a node
pub struct NodeHandle(pub(crate) Rc<RefCell<Node>>);

impl NodeHandle {
    /// Constructs a new handle from a node
    pub fn new(node: Node) -> Self {
        Self(Rc::new(RefCell::new(node)))
    }

    #[inline(always)]
    fn inner(&self) -> &Rc<RefCell<Node>> {
        &self.0
    }

    /// Immutably borrows the node
    pub fn borrow(&self) -> std::cell::Ref<Node> {
        self.inner().borrow()
    }

    /// Mutably borrows the node
    pub fn borrow_mut(&self) -> std::cell::RefMut<Node> {
        self.inner().borrow_mut()
    }

    /// Adds a child node to the current node.
    pub fn add_child_node(&self, mut child: Node) {
        child.parent = Some(Self(Rc::clone(self.inner())));
        let child = NodeHandle::new(child);

        self.borrow_mut().children.push(child);
    }

    /// Adds a child node handle to the current node.
    pub fn add_child(&self, child: NodeHandle) {
        child.borrow_mut().parent = Some(Self(Rc::clone(self.inner())));
        self.borrow_mut().children.push(child);
    }
}
