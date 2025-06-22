pub mod action;
mod context;
mod event;
mod hitmap;
mod viewport;

pub use action::Action;
pub use context::{AppContext, Context};
pub use event::Event;
pub use hitmap::HitMap;
pub use viewport::Viewport;

use std::{cell::RefCell, io, rc::Rc, time::Duration};

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

    context: AppContext,
    should_resize: Option<(u16, u16)>,
}

impl App {
    /// Registers a panic hook to cleanup the terminal state. This function doesn't replace any
    /// existing panic hook, it extends it with `take_hook()` and then `set_hook()`.
    ///
    /// Without calling this, you will not see any panic messages while in an `AlternateScreen`
    pub fn register_panic_hook() {
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            // Cleanup terminal state
            let _ = disable_raw_mode();
            let _ = execute!(io::stdout(), LeaveAlternateScreen);

            // Call the original panic hook
            hook(panic_info);
        }));
    }

    pub fn new(root: NodeHandle) -> Self {
        let context = AppContext::new(&root);

        App {
            quit_on: Some((KeyCode::Char('c'), KeyModifiers::CONTROL)),
            raw: true,
            alternate: true,
            root,
            hitmap: HitMap::default(),
            canvas: Canvas::default(),
            viewport: Viewport::new(),

            context,
            should_resize: None,
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

    /// Recomputes and renders the application based on `self.should_resize`.
    pub fn resize(&mut self) -> io::Result<()> {
        let Some((width, height)) = self.should_resize.take() else {
            return Ok(());
        };

        self.viewport.max = (width, height);
        self.viewport.screen = (width, height);
        self.context.screen_size = self.viewport.screen;
        self.canvas = Canvas::new(width as usize, height as usize);
        self.hitmap.resize(width, height);

        self.root
            .borrow_mut()
            .compute(Offset::default(), Size::from_cells(width, height));
        self.render()
    }

    /// Renders the application to the terminal.
    pub fn render(&mut self) -> io::Result<()> {
        self.root
            .borrow()
            .render_to(self.viewport, &mut self.canvas, &mut self.hitmap);
        self.canvas.prune_redundant_codes();
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
    pub fn handle_event(&mut self, event: Event) -> io::Result<()> {
        match event {
            Event::Key(event) => {
                self.dispatch_key_event(event);
            }
            Event::Mouse(mouse_event) => {
                self.dispatch_mouse_event(mouse_event);
                self.should_resize = Some(self.viewport.screen); // just for debug, remove later
            }
            Event::TerminalResize(width, height) => {
                self.should_resize = Some((width, height));
                println!("Resize {width}x{height}")
            }
            Event::Paste(paste) => self.dispatch_paste_event(paste),

            event => println!("{event:?}"),
        }

        Ok(())
    }

    /// Runs the main application loop.
    pub fn run(&mut self) -> io::Result<()> {
        self.prepare_screen()?;
        self.should_resize = Some(self.viewport.screen);

        loop {
            let mut render = false;

            while crossterm::event::poll(Duration::from_millis(0))? {
                let event = crossterm::event::read()?;
                let event = Event::from_crossterm_event(event);

                if let Some(e) = event.as_key_event() {
                    if let Some((code, modifiers)) = self.quit_on {
                        if e.code == code && e.modifiers.contains(modifiers) {
                            return Ok(());
                        }
                    }
                }

                self.handle_event(event)?;
                render = true;
            }

            timed(|| self.resize())?;

            if render {
                timed(|| self.render())?;
            }
        }
    }

    /// Returns the path from the target node `id` to the root node.
    /// TODO: temporary solution, remove in the future
    pub fn get_path_to(&self, id: NodeId) -> Option<Vec<(Rc<RefCell<Node>>, WeakNodeHandle)>> {
        let mut path = Vec::new();

        if self.root.borrow().build_path_to_node(id, &mut path) {
            path.push((self.root.0.clone(), self.root.weak()));
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

    /// Dispatches an event to the target node in capture, target and bubble phases
    pub fn dispatch_event(&mut self, event: Event, target_id: NodeId) {
        let Some(path) = self.get_path_to(target_id) else {
            return;
        };

        let Some((target, target_weak)) = path.first() else {
            return;
        };

        // Set focus to target node on mouse down
        if let Some(mouse_event) = event.as_mouse_event() {
            if mouse_event.kind.is_down() {
                self.context.focus = Some((target_id, target_weak.clone()));
            }
        }

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
