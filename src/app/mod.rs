pub mod action;
mod context;
mod event;
pub mod focus;
mod hitmap;
mod viewport;

pub use action::Action;
pub use context::{AppContext, Context};
pub use event::Event;
pub use hitmap::HitMap;
pub use viewport::Viewport;

use std::{
    cell::RefCell,
    io,
    rc::Rc,
    time::{Duration, Instant},
};

use crossterm::{
    event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind},
    execute,
    terminal::{LeaveAlternateScreen, disable_raw_mode},
};

use crate::*;

pub struct App {
    pub quit_on: Option<(KeyCode, KeyModifiers)>,
    raw: bool,
    alternate: bool,
    root: NodeHandle,

    hitmap: HitMap,
    canvas: Canvas,
    viewport: Viewport,

    pub(crate) context: AppContext,
    should_resize: Option<(u16, u16)>,
    should_compute: bool,
    should_render: bool,
    should_draw: bool,
    should_quit: bool,
}

impl App {
    /// Registers a panic hook to cleanup the terminal state. This function doesn't replace any
    /// existing panic hook, it extends it with `take_hook()` and then `set_hook()`.
    ///
    /// Without calling this, you will not see any panic messages while in an `AlternateScreen`
    pub fn register_panic_hook() {
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            while crossterm::event::poll(Duration::from_millis(0)).unwrap_or_default() {
                let _ = crossterm::event::read();
            }

            // Cleanup terminal state
            let _ = execute!(io::stdout(), LeaveAlternateScreen);
            let _ = disable_raw_mode();

            // Call the original panic hook
            hook(panic_info);
        }));
    }

    pub fn new(root: NodeHandle) -> Self {
        let (width, height) = crossterm::terminal::size().unwrap_or_default();
        let context = AppContext::new(&root, (width, height));

        App {
            quit_on: Some((KeyCode::Char('c'), KeyModifiers::CONTROL)),
            raw: true,
            alternate: true,
            root,

            hitmap: HitMap::new(width as usize, height as usize),
            canvas: Canvas::new(width as usize, height as usize),
            viewport: Viewport::new(width, height),

            context,
            should_resize: None,
            should_compute: false,
            should_render: false,
            should_draw: false,
            should_quit: false,
        }
    }

    /// Prepares the terminal screen based on the application settings.
    fn prepare_screen(&mut self) -> io::Result<()> {
        use crossterm::event::{EnableBracketedPaste, EnableFocusChange, EnableMouseCapture};

        if self.alternate {
            execute!(
                io::stdout(),
                crossterm::terminal::EnterAlternateScreen,
                EnableMouseCapture,
                EnableFocusChange,
                EnableBracketedPaste,
            )?
        }

        if self.raw {
            crossterm::terminal::enable_raw_mode()?
        }

        Ok(())
    }

    /// Prepares the application for resizing, causes full recompute of the layout
    pub fn resize(&mut self) -> io::Result<()> {
        let Some((width, height)) = self.should_resize.take() else {
            return Ok(());
        };

        self.viewport.max = (width, height);
        self.viewport.screen = (width, height);
        self.context.screen_size = self.viewport.screen;
        self.canvas = Canvas::new(width as usize, height as usize);
        self.hitmap.resize(width, height);

        self.should_compute = true;
        Ok(())
    }

    /// Recomputes the application layout if `self.should_compute` is true. Causes a full render
    pub fn compute(&mut self) -> io::Result<()> {
        if !self.should_compute {
            return Ok(());
        }
        self.should_compute = false;

        let (width, height) = self.viewport.screen;

        self.root
            .borrow_mut()
            .compute(Offset::default(), Size::from_cells(width, height));

        self.should_render = true;
        Ok(())
    }

    /// Renders the application to the canvas and hitmap if `self.should_render` is true. Causes a
    /// full redraw
    pub fn render(&mut self) -> io::Result<()> {
        if !self.should_render {
            return Ok(());
        }
        self.should_render = false;

        self.root
            .borrow()
            .render_to(self.viewport, &mut self.canvas, &mut self.hitmap);
        self.canvas.prune_redundant_codes();

        self.should_draw = true;
        Ok(())
    }

    /// Draws the application to the terminal if `self.should_draw` is true.
    pub fn draw(&mut self) -> io::Result<()> {
        if !self.should_draw {
            return Ok(());
        }
        self.should_draw = false;

        self.canvas.render()?;
        // self.hitmap.debug_render();
        self.move_cursor_to_focus()?;

        Ok(())
    }

    /// Moves the cursor to the focus position in the terminal.
    pub fn move_cursor_to_focus(&mut self) -> io::Result<()> {
        let Some((_, ref focus_weak)) = self.context.focus else {
            return Ok(());
        };

        let Some(focus) = focus_weak.upgrade() else {
            return Ok(());
        };

        let focus = focus.borrow();
        let (cursor_x, cursor_y) = focus.focus_cursor_position();
        execute!(io::stdout(), crossterm::cursor::MoveTo(cursor_x, cursor_y))
    }

    /// Handles an event, dispatching it to the target node if applicable.
    pub fn handle_crossterm_event(&mut self, event: crossterm::event::Event) -> io::Result<()> {
        if let Some(key_event) = event.as_key_event() {
            if self.should_quit(&key_event) {
                return Ok(());
            }
        }

        use crossterm::event::Event as CEvent;
        match event {
            CEvent::Key(key_event) => self.dispatch_key_event(key_event),
            CEvent::Mouse(mouse_event) => {
                self.dispatch_mouse_event(mouse_event);
                // self.should_resize = Some(self.viewport.screen); // just for debug, remove later
            }
            CEvent::Resize(width, height) => {
                self.should_resize = Some((width, height));
                println!("Resize {width}x{height}")
            }
            CEvent::Paste(paste) => self.dispatch_paste_event(paste),

            event => println!("{event:?}"),
        }

        Ok(())
    }

    /// Checks if the app should quit based on the key event, or if `self.should_quit == true`.
    /// Sets `self.should_quit` to true if the `quit_on` condition is met.
    pub fn should_quit(&mut self, event: &crossterm::event::KeyEvent) -> bool {
        if self.should_quit {
            return true;
        }

        if let Some((code, modifiers)) = self.quit_on {
            if event.code == code && event.modifiers.contains(modifiers) {
                self.should_quit = true;
                return true;
            }
        }
        false
    }

    /// Runs the main application loop.
    pub fn run(&mut self) -> io::Result<()> {
        self.prepare_screen()?;
        self.should_resize = Some(self.viewport.screen);

        let mut cleanup_time = Instant::now();
        let mut dynamic_timeout = DynamicTimeout::new(0.1, 1.0);

        // Init the worker channel
        let receiver = workers::init_channel();
        self.execute_queued_workers();

        loop {
            // Cleanup workers every 10 seconds
            self.periodic_workers_cleanup(&mut cleanup_time, 10);

            // Poll for events without blocking, using dynamic timeout
            while crossterm::event::poll(dynamic_timeout.get())? {
                let event = crossterm::event::read()?;
                self.handle_crossterm_event(event)?;
                dynamic_timeout.update();
            }

            // Receive messages without blocking
            if self.handle_messages(&receiver)? {
                dynamic_timeout.update();
            }

            // Drain the actions queue
            if !self.context.actions.is_empty() {
                self.handle_actions()?;
                dynamic_timeout.update();
            }

            // Check if we should quit
            if self.should_quit {
                return Ok(());
            }

            self.resize()?;
            self.compute()?;

            self.render()?;
            self.draw()?;
        }
    }


    /// Find a node by its [`id`](NodeId), returns its `weak handle` if found in the tree.
    /// The weak handle is guaranteed to be valid when returned
    pub fn get_weak_by_id(&self, id: NodeId) -> Option<WeakNodeHandle> {
        fn find_recursive(weak: WeakNodeHandle, node: &Node, id: NodeId) -> Option<WeakNodeHandle> {
            if node.id() == id {
                return Some(weak);
            }

            node.children
                .iter()
                .find_map(|n| find_recursive(n.weak(), &n.borrow(), id))
        }
        find_recursive(self.root.weak(), &self.root.borrow(), id)
    }

    /// Returns the path from the target node `id` to the root node.
    /// TODO: temporary solution, remove in the future
    pub fn get_path_to(&self, id: NodeId) -> Option<Vec<(Rc<RefCell<Node>>, WeakNodeHandle)>> {
        let mut path = Vec::new();

        if self.root.borrow().build_path_to_node(id, &mut path) {
            path.push((self.root.inner().clone(), self.root.weak()));
            Some(path)
        } else {
            None
        }
    }

    /// Dispatches a paste event to the target node based on current focus.
    pub fn dispatch_paste_event(&mut self, paste: String) {
        let Some((focus_id, _)) = self.context.focus else {
            return;
        };

        self.dispatch_event(Event::Paste(paste), focus_id);
    }

    /// Dispatches a key event to the target node based on current focus.
    pub fn dispatch_key_event(&mut self, key_event: KeyEvent) {
        let Some((focus_id, _)) = self.context.focus else {
            return;
        };

        self.dispatch_event(Event::Key(key_event), focus_id);
    }

    /// Dispatches a mouse event to the target node based on the hitmap.
    pub fn dispatch_mouse_event(&mut self, mouse_event: MouseEvent) {
        let (column, row) = (mouse_event.column, mouse_event.row);

        let Some(mut target_id) = self.hitmap.get(column, row) else {
            return;
        };

        // Handle hold, and replace target_id if dragging
        match mouse_event.kind {
            MouseEventKind::Down(_) => self.context.hold = Some((column, row, target_id)),
            MouseEventKind::Up(_) => self.context.hold = None,
            MouseEventKind::Drag(_) => {
                if let Some((_, _, old_target_id)) = self.context.hold {
                    target_id = old_target_id;
                }
            }
            _ => {}
        }

        self.dispatch_event(Event::Mouse(mouse_event), target_id);
    }

    /// Dispatches a `focus lost` and `focus gained` event to the relevant nodes. Parents with
    /// `focus within` will receive an event only if the resulting change affects them.
    pub fn dispatch_node_focus_event(
        &mut self,
        new_focus_id: NodeId,
        new_focus_weak: WeakNodeHandle,
    ) {
        let old_focus = self.context.focus.replace((new_focus_id, new_focus_weak));

        let Some((old_focus_id, _)) = old_focus else {
            // If there is no old focus, dispatch only the focus gained event
            self.dispatch_event(Event::NodeFocusGained, new_focus_id);
            return;
        };

        let Some(old_path) = self.get_path_to(old_focus_id) else {
            return;
        };
        let Some(new_path) = self.get_path_to(new_focus_id) else {
            return;
        };

        // Get the last common parent index of the old and new focus paths
        let mut last_common_parent_index = 0;
        for (old, new) in old_path.iter().rev().zip(new_path.iter().rev()) {
            if Rc::ptr_eq(&old.0, &new.0) {
                last_common_parent_index += 1;
            } else {
                break;
            }
        }

        // Dispatch focus lost events to the differing parents
        let old_path = &old_path[..old_path.len() - last_common_parent_index];
        self.execute_event_phases(Event::NodeFocusLost, old_path);

        // Dispatch focus gained events to the differing parents
        let new_path = &new_path[..new_path.len() - last_common_parent_index];
        self.execute_event_phases(Event::NodeFocusGained, new_path);
    }

    /// Dispatches an event to the target node in capture, target and bubble phases
    pub fn dispatch_event(&mut self, event: Event, target_id: NodeId) {
        let Some(path) = self.get_path_to(target_id) else {
            return;
        };

        let Some((_, target_weak)) = path.first() else {
            return;
        };

        // Set focus to target node on mouse down
        if let Some(mouse_event) = event.as_mouse_event() {
            if mouse_event.kind.is_down() {
                self.dispatch_node_focus_event(target_id, target_weak.clone());
            }
        }

        // Execute the event phases
        self.execute_event_phases(event, &path);
    }

    /// Executes the event phases (capture, target, bubble) for the given event and target node.
    /// You must provide a path for the execution.
    ///
    /// # Usage
    /// `path` must contain the target node as the **first element** and the root node as the
    /// **last element**. It can be obtained using [`self.get_path_to`](Self::get_path_to).
    ///
    /// # Safety
    /// No borrows of nodes in the path should be held while calling this method.
    fn execute_event_phases(&mut self, event: Event, path: &[(Rc<RefCell<Node>>, WeakNodeHandle)]) {
        let Some((target, target_weak)) = path.first() else {
            // No target node in the path, nothing to do
            return;
        };
        let target_id = target.borrow().id();

        // Create event handler context
        let mut context = Context::new(
            &mut self.context,
            target_id,
            event,
            target_weak.clone(),
            target_weak.clone(),
        );

        // Capture phase
        for (node, weak) in path.iter().skip(1).rev() {
            let mut node = node.borrow_mut();
            context.self_weak = weak.clone();
            if node.handle_event(&mut context, true) {
                return;
            }
        }

        // Target phase
        {
            let mut node = target.borrow_mut();
            context.self_weak = target_weak.clone();
            context.is_target_phase = true;

            let capture = node.handle_event(&mut context, true);
            let bubble = node.handle_event(&mut context, false);
            context.is_target_phase = false;

            if capture || bubble {
                return;
            }
        }

        // Bubble phase
        for (node, weak) in path.iter().skip(1) {
            let mut node = node.borrow_mut();
            context.self_weak = weak.clone();
            if node.handle_event(&mut context, false) {
                return;
            }
        }
    }

    /// Traverses the root and executes all queued workers, this must be run after worker channel
    /// initialization by calling [workers::init_channel]
    fn execute_queued_workers(&mut self) {
        fn traverse(node: &mut Node) {
            node.workers.execute_queue();

            for child in &node.children {
                traverse(&mut child.borrow_mut())
            }
        }
        traverse(&mut self.root.borrow_mut());
    }

    /// Periodically cleanup workers
    fn periodic_workers_cleanup(&mut self, time: &mut Instant, secs: u64) {
        if time.elapsed() < Duration::from_secs(secs) {
            return;
        }
        *time = Instant::now();

        fn traverse(node: &mut Node) {
            node.workers.cleanup();

            for child in &node.children {
                traverse(&mut child.borrow_mut())
            }
        }
        traverse(&mut self.root.borrow_mut());
    }
}

impl Drop for App {
    fn drop(&mut self) {
        use crossterm::event::{DisableBracketedPaste, DisableFocusChange, DisableMouseCapture};

        if self.alternate {
            execute!(
                io::stdout(),
                LeaveAlternateScreen,
                DisableMouseCapture,
                DisableFocusChange,
                DisableBracketedPaste,
            )
            .expect("Failed to leave alternate screen");
        }

        if self.raw {
            disable_raw_mode().expect("Failed to disable raw mode");
        }
    }
}

#[derive(Debug, Clone)]
/// A dynamic timeout that adjusts based on the time elapsed since the last activity.
/// If idle for too long, the timeout will be set to the maximum value, if the time since the last
/// activity is less than `max_idle_secs`, the timeout will be dynamically computed.
pub struct DynamicTimeout {
    /// The maximum timeout duration in seconds
    max_timeout_secs: f32,
    /// The maximum idle time before the timeout is capped at `max_timeout_secs`
    max_idle_secs: f32,
    /// The last time of activity (not idle)
    last_activity: Instant,
}

impl DynamicTimeout {
    /// Creates a new `DynamicTimeout`
    pub fn new(max_timeout_secs: f32, max_idle_secs: f32) -> Self {
        Self {
            max_timeout_secs,
            max_idle_secs,
            last_activity: Instant::now(),
        }
    }

    /// Updates the timeout, marking the current time as the last activity.
    pub fn update(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Returns the computed timeout duration.
    pub fn get(&self) -> Duration {
        let elapsed_secs = self.last_activity.elapsed().as_secs_f32();
        let dynamic_timeout_secs =
            (elapsed_secs / self.max_idle_secs).powi(2).min(1.0) * self.max_timeout_secs;
        let dynamic_timeout_ms = (dynamic_timeout_secs * 1000.0) as u64; // Convert to millis

        Duration::from_millis(dynamic_timeout_ms)
    }
}
