use std::{
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SendError, Sender},
    },
    thread::{self, JoinHandle},
};

use crate::{AppContext, Node, NodeId};

#[derive(Default, Debug)]
/// Worker threads for a node, marks them for shutdown on drop
pub struct Workers {
    /// Marks a cooperative shutdown to all threads
    shutdown: Arc<AtomicBool>,
    /// Thread handles
    handles: Vec<JoinHandle<()>>,
}

/// Context inside a [WorkerFn]
/// Should be used to check for an early exit via [WorkerContext::is_shutdown]
pub struct WorkerContext {
    /// Channel sender for this worker context
    sender: Sender<Message>,
    /// Shutdown flag from [Workers]
    shutdown: Arc<AtomicBool>,
    /// The NodeId to which this worker is attatched
    pub node_id: NodeId,
}

impl WorkerContext {
    /// True if the thread is marked for shutdown, the thread should exit
    #[inline]
    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::Relaxed)
    }

    /// Send a message to the main [`app`](crate::App) loop
    #[inline]
    pub fn send(&self, message: Message) -> Result<(), SendError<Message>> {
        self.sender.send(message)
    }
}

/// Message returned from a [`thread worker`](Workers)
pub enum Message {
    // Action(Action),
    Exec(Box<dyn FnOnce(&mut AppContext, &mut Node) -> () + Send + 'static>),
}

// static COM: (Sender<usize>, Receiver<usize>) = mpsc::channel();
static WORKER_SENDER: OnceLock<Sender<Message>> = OnceLock::new();

/// Initialize the [`channel`](mpsc) for threads, done automatically in [app](crate::App)
/// # Panics
/// Panics if called again
pub fn init_channel() -> Receiver<Message> {
    let (sender, receiver) = mpsc::channel();
    WORKER_SENDER.set(sender).unwrap();
    receiver
}

/// Worker funciton type used in [Workers]
pub trait WorkerFn: FnMut(WorkerContext) -> () + Send + 'static {}

impl Workers {
    // Start a new worker thread
    pub fn start(&mut self, mut f: impl WorkerFn, node_id: NodeId) {
        let context = WorkerContext {
            sender: WORKER_SENDER.get().unwrap().clone(),
            shutdown: Arc::clone(&self.shutdown),
            node_id,
        };

        let handle = thread::spawn(move || {
            f(context);

            // println!("Worker thread for node {:?} shutting down", node_id);
        });

        self.handles.push(handle);
    }
}

// When Node is dropped, signal shutdown and optionally detach thread
impl Drop for Workers {
    fn drop(&mut self) {
        // Signal the threads to stop
        self.shutdown.store(true, Ordering::Relaxed);
    }
}
