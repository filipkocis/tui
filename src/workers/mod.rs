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
    /// The total amount of threads started
    count: usize,
    /// Marks a cooperative shutdown to all threads
    shutdown: Arc<AtomicBool>,
    /// Thread handles
    handles: Vec<JoinHandle<()>>,
    /// Workers awaiting channel connection
    queue: Vec<Box<dyn WorkerFn>>,
}

/// Worker function used in the thread execution
pub trait WorkerFn: FnOnce(WorkerContext) + Send + 'static {}
impl<T> WorkerFn for T where T: FnOnce(WorkerContext) + Send + 'static {}

impl Debug for dyn WorkerFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("WorkerFn").finish_non_exhaustive()
    }
}

/// Context inside a [WorkerFn]
/// Should be used to check for an early exit via [WorkerContext::is_shutdown].
/// To send a message back to the main loop/thread use [`self.send(message)`][WorkerContext::send]
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
            count: 0,
            shutdown: Arc::default(),
            handles: Vec::default(),
            queue: Vec::default(),
        }
    }

    /// Start a new worker thread
    /// # Note
    /// If the channel is not yet initialized, `f` will be put into a queue
    pub fn start(&mut self, f: impl WorkerFn) {
        let Some(sender) = WORKER_SENDER.get().cloned() else {
            self.queue.push(Box::new(f));
            return;
        };
        self.count += 1;
        let node_id = self.node_id;
        let context = WorkerContext {
            sender,
            shutdown: Arc::clone(&self.shutdown),
            node_id,
        };

        let thread_no = self.count;
        let thread_name = format!("No.{thread_no} for {:?}", self.node_id);
        let builder = thread::Builder::new().name(thread_name.clone());

        let handle = builder
            .spawn(move || {
                // Catch the panic so it doesn't panic on join in main thread
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    // Run worker fn
                    f(context);

                    // Log end of thread
                    info!("Worker thread {thread_name} shutting down");
                }));
            })
            .expect("Could not create worker thread");

        self.handles.push(handle);
        self.cleanup();
    }

    /// Execute the queued workers, will do nothing if the channel is still not initialized
    pub fn execute_queue(&mut self) {
        let queue = self.queue.drain(..).collect::<Vec<_>>();

        for f in queue {
            self.start(f);
        }
    }

    /// Removes finished threads from handles vec
    pub fn cleanup(&mut self) {
        let mut removed = Vec::new();
        for i in (0..self.handles.len()).rev() {
            if self.handles[i].is_finished() {
                removed.push(self.handles.swap_remove(i))
            }
        }

        for t in removed {
            let tid = t.thread().id();
            let tname = t.thread().name().unwrap_or_default().to_string();
            if t.join().is_err() {
                // Workers are wrapped with catch_unwind so this should almost never happen
                panic!("uncaught worker thread ({tid:?}) {tname:?} panicked at join");
            }
        }
    }
}

impl Drop for Workers {
    fn drop(&mut self) {
        // Signal the threads to stop
        self.shutdown.store(true, Ordering::Relaxed);
    }
}
