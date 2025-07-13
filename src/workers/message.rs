use std::sync::mpsc::Receiver;

use crate::{App, AppContext, Node, NodeId};

/// Message returned from a [`thread worker`](Workers)
pub enum Message {
    Exec(Box<dyn FnOnce(&mut AppContext, &mut Node) -> () + Send + 'static>),
}

/// Message returned from a [`thread worker`](Workers) wrapped with its `NodeId`. Used internally,
/// preffer to use [Message]
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
                    let mut node = node.borrow_mut();
                    f(&mut self.context, &mut node);
                }
            }
        };

        Ok(())
    }
}
