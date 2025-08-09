use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
    sync::mpsc::Receiver,
};

use crate::{App, AppContext, Node, NodeId, WeakNodeHandle};

/// Message sent from a [`thread worker`](Workers) back to the main thread loop via a channel
pub enum Message {
    /// Function which gets executed on the main thread after receiving the message
    Exec(Box<dyn FnOnce(ExecContext) + Send + 'static>),
}

impl Message {
    /// Creates a [Message::Exec] but without the need to wrap it with `box`
    #[inline]
    pub fn exec(f: impl FnOnce(ExecContext) + Send + 'static) -> Self {
        Self::Exec(Box::new(f))
    }
}

/// Context used for a [Message::Exec] sent from a [`thread worker`](crate::workers::Workers)
pub struct ExecContext<'a> {
    /// Mutable app reference
    app: &'a mut App,
    /// Current weak node handle, will **not** panic on upgrade
    pub self_weak: WeakNodeHandle,
    /// Current node, unwrapped from its handle
    node: Rc<RefCell<Node>>,
}

impl<'a> ExecContext<'a> {
    /// Creates new message exec context
    fn new(app: &'a mut App, node: Rc<RefCell<Node>>) -> Self {
        Self {
            app,
            self_weak: WeakNodeHandle::new(Rc::downgrade(&node)),
            node,
        }
    }

    /// Get the [`app context`](AppContext)
    #[inline]
    pub fn app(&self) -> &AppContext {
        &self.app.context
    }

    /// Get the [`app context`](AppContext) mutably
    #[inline]
    pub fn app_mut(&mut self) -> &mut AppContext {
        &mut self.app.context
    }

    /// Borrows `self.node`, ergonomic `self_weak` borrow
    #[inline]
    pub fn node(&self) -> Ref<'_, Node> {
        self.node.borrow()
    }

    /// Mutably borrows `self.node`, ergonomic `self_weak` borrow
    #[inline]
    pub fn node_mut(&mut self) -> RefMut<'_, Node> {
        self.node.borrow_mut()
    }

    /// Find a node by its `id`, returning a `weak handle` if found.
    /// # Panics
    /// If any node is mutably borrowed this will panic
    #[inline]
    pub fn find_node(&self, id: NodeId) -> Option<WeakNodeHandle> {
        self.app.get_weak_by_id(id)
    }
}

/// Message returned from a [`thread worker`](crate::workers::Workers) wrapped with its `NodeId`.
/// Used internally, preffer to use [Message].
pub type InternalMessage = (NodeId, Message);

/// Implementing structs can handle messages in their context.
pub trait MessageHandling {
    /// Handles a message in the context of the application (the implementing struct).
    fn handle_message(&mut self, message: InternalMessage) -> std::io::Result<()>;
}

impl App {
    /// Handles all messages in the channel. Returns `true` if any messages were handled
    pub fn handle_messages(
        &mut self,
        receiver: &Receiver<InternalMessage>,
    ) -> std::io::Result<bool> {
        let mut handled = false;

        while let Ok(message) = receiver.try_recv() {
            self.handle_message(message)?;
            handled = true;
        }

        Ok(handled)
    }
}

impl MessageHandling for App {
    fn handle_message(&mut self, message: InternalMessage) -> std::io::Result<()> {
        let (id, message) = message;

        match message {
            Message::Exec(f) => {
                if let Some(node) = self.get_weak_by_id(id).and_then(|w| w.upgrade()) {
                    f(ExecContext::new(self, node));
                }
            }
        };

        Ok(())
    }
}
