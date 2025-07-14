use std::sync::mpsc::Receiver;

use crate::{App, Node, NodeId};

/// Message sent from a [`thread worker`](Workers) back to the main thread loop via a channel
pub enum Message {
    /// Function which gets executed on the main thread after receiving the message
    Exec(Box<dyn FnOnce(&mut App, &mut Node) -> () + Send + 'static>),
}

impl Message {
    /// Creates a [Message::Exec] but without the need to wrap it with `box`
    #[inline]
    pub fn exec(f: impl FnOnce(&mut App, &mut Node) -> () + Send + 'static) -> Self {
        Self::Exec(Box::new(f))
    }
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
                    f(self, &mut node);
                }
            }
        };

        Ok(())
    }
}
