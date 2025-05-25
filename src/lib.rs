mod canvas;
mod char;
mod context;
mod elements;
mod handler;
mod hitmap;
mod line;
mod node;
mod style;
mod viewport;

pub use canvas::Canvas;
pub use char::{Char, Code};
pub use context::{AppContext, Context};
pub use elements::*;
pub use handler::{EventHandlers, IntoEventHandler};
pub use hitmap::HitMap;
pub use line::Line;
pub use node::{text, Node, NodeHandle, NodeId, WeakNodeHandle};
pub use style::{Offset, Padding, Size, SizeValue, Style};
pub use viewport::Viewport;

use std::{
    cell::RefCell,
    io::{self, stdin, Read},
    rc::Rc,
    time::Duration,
};

use crossterm::{
    self,
    event::{
        self, DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
        EnableFocusChange, EnableMouseCapture, Event, KeyEvent, MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

pub struct App {
    raw: bool,
    alternate: bool,
    root: NodeHandle,

    hitmap: HitMap,
    canvas: Canvas,
    viewport: Viewport,

    context: AppContext,
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
        App {
            raw: true,
            alternate: true,
            root,
            hitmap: HitMap::default(),
            canvas: Canvas::default(),
            viewport: Viewport::new(),

            context: AppContext::default(),
        }
    }

    fn prepare_screen(&mut self) -> io::Result<()> {
        if self.alternate {
            execute!(
                io::stdout(),
                EnterAlternateScreen,
                EnableMouseCapture,
                EnableFocusChange,
                EnableBracketedPaste,
            )?
        }

        if self.raw {
            enable_raw_mode()?
        }

        Ok(())
    }

    pub fn resize(&mut self, width: u16, height: u16) -> io::Result<()> {
        self.viewport.max = (width, height);
        self.viewport.screen = (width, height);
        self.canvas = Canvas::new(width as usize, height as usize);
        self.hitmap.resize(width, height);

        self.root
            .borrow_mut()
            .compute(Offset::default(), Size::from_cells(width, height));
        self.render()
    }

    pub fn render(&mut self) -> io::Result<()> {
        self.root
            .borrow()
            .render_to(self.viewport, &mut self.canvas, &mut self.hitmap);
        self.canvas.prune_redundant_codes();
        self.canvas.render()?;
        self.move_cursor_to_focus()?;
        Ok(())
    }

    pub fn move_cursor_to_focus(&mut self) -> io::Result<()> {
        let Some((_, ref focus_weak)) = self.context.focus else {
            return Ok(());
        };

        let Some(focus) = focus_weak.upgrade() else {
            return Ok(());
        };

        let focus = focus.borrow();
        let Some((cursor_x, cursor_y)) = focus.absolute_cursor_position() else {
            return Ok(());
        };

        execute!(io::stdout(), crossterm::cursor::MoveTo(cursor_x, cursor_y))
    }

    pub fn run(&mut self) -> io::Result<()> {
        self.prepare_screen()?;
        self.resize(self.viewport.screen.0, self.viewport.screen.1)?;

        loop {
            let mut resize = None;
            let mut render = false;

            while event::poll(Duration::from_millis(0))? {
                let event = event::read()?;

                use event::*;
                match event {
                    Event::Key(event) => {
                        self.dispatch_key_event(event);
                        match event.code {
                            KeyCode::Esc => {
                                return Ok(());
                            }
                            event => println!("{event:?}"),
                        }
                    }
                    Event::Mouse(mouse_event) => {
                        self.dispatch_mouse_event(mouse_event);
                        resize = Some(self.viewport.screen); // just for debug, remove later
                    }
                    Event::Resize(width, height) => {
                        resize = Some((width, height));
                        println!("Resize {width}x{height}")
                    }
                    Event::Paste(paste) => self.dispatch_paste_event(paste),

                    event => println!("{event:?}"),
                }

                render = true;
            }

            if let Some((width, height)) = resize.take() {
                self.resize(width, height)?;
            }

            if render {
                self.render()?;
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
