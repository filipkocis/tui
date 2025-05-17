mod canvas;
mod char;
mod elements;
mod handler;
mod hitmap;
mod line;
mod node;
mod style;
mod viewport;

pub use canvas::Canvas;
pub use char::{Char, Code};
pub use elements::*;
pub use handler::{EventHandlers, IntoEventHandler};
pub use hitmap::HitMap;
pub use line::Line;
pub use node::{text, Node, NodeHandle, NodeId, WeakNodeHandle};
pub use style::{Offset, Padding, Size, SizeValue, Style};
pub use viewport::Viewport;

use std::{cell::RefCell, io, rc::Rc, time::Duration};

use crossterm::{
    self,
    event::{
        self, DisableFocusChange, DisableMouseCapture, EnableFocusChange, EnableMouseCapture, Event,
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
}

impl App {
    pub fn new(root: NodeHandle) -> Self {
        App {
            raw: true,
            alternate: true,
            root,
            hitmap: HitMap::default(),
            canvas: Canvas::default(),
            viewport: Viewport::new(),
        }
    }

    fn prepare_screen(&mut self) -> io::Result<()> {
        if self.alternate {
            execute!(
                io::stdout(),
                EnterAlternateScreen,
                EnableMouseCapture,
                EnableFocusChange
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
        Ok(())
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
                    Event::Key(event) => match event.code {
                        KeyCode::Esc => {
                            return Ok(());
                        }
                        event => println!("{event:?}"),
                    },
                    Event::Mouse(_event) => {
                        self.dispatch_mouse_event(&event);
                    }
                    Event::Resize(width, height) => {
                        resize = Some((width, height));
                        println!("Resize {width}x{height}")
                    }
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
    pub fn get_path_to(&self, id: NodeId) -> Option<Vec<Rc<RefCell<Node>>>> {
        let mut path = Vec::new();

        if self.root.borrow().build_path_to_node(id, &mut path) {
            path.push(self.root.0.clone());
            Some(path)
        } else {
            None
        }
    }

    /// Dispatches a mouse event to the target node based on the hitmap.
    pub fn dispatch_mouse_event(&mut self, event: &Event) {
        let (column, row) = match event {
            Event::Mouse(event) => (event.column, event.row),
            _ => return,
        };

        let Some(target) = self.hitmap.get(column, row) else {
            return;
        };

        let Some(path) = self.get_path_to(target) else {
            return;
        };

        let Some(target) = path.first() else {
            return;
        };

        // Capture phase
        for node in path.iter().skip(1).rev() {
            let mut node = node.borrow_mut();
            if node.handle_event(event, true) {
                return;
            }
        }

        // Target phase
        {
            let mut node = target.borrow_mut();
            let capture = node.handle_event(event, true);
            let bubble = node.handle_event(event, false);
            if capture || bubble {
                return;
            }
        }

        // Bubble phase
        for node in path.iter().skip(1) {
            let mut node = node.borrow_mut();
            if node.handle_event(event, false) {
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
                DisableFocusChange
            )
            .expect("Failed to leave alternate screen");
        }

        if self.raw {
            disable_raw_mode().expect("Failed to disable raw mode");
        }
    }
}
