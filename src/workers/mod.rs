pub mod message;

pub use message::Message;

use std::{
    fmt::Debug,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread::{self, JoinHandle},
};

use crate::{NodeId, workers::message::InternalMessage};

#[derive(Debug)]
/// Worker threads for a node, marks them for shutdown on drop
pub struct Workers {
    /// NodeID of the node which owns the workers
    node_id: NodeId,
    /// Marks a cooperative shutdown to all threads
    shutdown: Arc<AtomicBool>,
    /// Thread handles
    handles: Vec<JoinHandle<()>>,

/// Worker function used in the thread execution
pub trait WorkerFn: FnOnce(WorkerContext) + Send + 'static {}
impl<T> WorkerFn for T where T: FnOnce(WorkerContext) + Send + 'static {}

impl Debug for dyn WorkerFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("WorkerFn").finish_non_exhaustive()
    }
}

/// Context inside a [WorkerFn]
/// Should be used to check for an early exit via [WorkerContext::is_shutdown]
pub struct WorkerContext {
    /// Channel sender for this worker context
    sender: Sender<InternalMessage>,
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
    pub fn send(&self, message: Message) -> Result<(), Message> {
        self.sender.send((self.node_id, message)).map_err(|e| e.0.1)
    }
}

/// Global sender used for worker channel communication via [`messages`](Message) with the [NodeId]
/// of their owning node
pub static WORKER_SENDER: OnceLock<Sender<InternalMessage>> = OnceLock::new();

/// Initialize the [`channel`](mpsc) for threads, done automatically in
/// [`app.run()`](crate::App::run)
/// # Panics
/// Panics if called again
pub fn init_channel() -> Receiver<InternalMessage> {
    let (sender, receiver) = mpsc::channel();
    WORKER_SENDER.set(sender).unwrap();
    receiver
}

impl Workers {
    /// Returns new workers for `NodeId`
    pub fn new(id: NodeId) -> Self {
        Self {
            node_id: id,
            shutdown: Arc::default(),
            handles: Vec::default(),
        }
    }

    /// Start a new worker thread
    /// # Note
    /// If the channel is not yet initialized, `f` will be put into a queue
    pub fn start(&mut self, f: impl WorkerFn) {
        let node_id = self.node_id;
        let context = WorkerContext {
            sender: WORKER_SENDER.get().unwrap().clone(),
            shutdown: Arc::clone(&self.shutdown),
            node_id,
        };

        let handle = thread::spawn(move || {
            f(context);

            // let str = format!("Worker thread for node {node_id:?} shutting down");
            // crate::Console::print(str);
        });

        self.handles.push(handle);
    }
}

impl Drop for Workers {
    fn drop(&mut self) {
        // Signal the threads to stop
        self.shutdown.store(true, Ordering::Relaxed);
    }
}
