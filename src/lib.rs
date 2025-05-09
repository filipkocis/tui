mod canvas;
mod line;
mod node;
mod offset;
mod style;
mod viewport;

pub use canvas::Canvas;
pub use line::{Char, Code, Line};
pub use node::Node;
pub use offset::Offset;
pub use style::Style;
pub use viewport::Viewport;

use std::{cell::RefCell, io, rc::Rc, time::Duration};

use crossterm::{
    self,
    event::{self, DisableFocusChange, DisableMouseCapture, EnableFocusChange, EnableMouseCapture},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};

pub struct App {
    raw: bool,
    alternate: bool,
    root: Rc<RefCell<Node>>,
    viewport: Viewport,
}

impl App {
    pub fn new(root: Node) -> Self {
        App {
            raw: true,
            alternate: true,
            root: Rc::new(RefCell::new(root)),
            viewport: Viewport::new(),
        }
    }

    pub fn add_child(&mut self, child: Node) {
        let child = Rc::new(RefCell::new(child));
        child.borrow_mut().parent = Some(self.root.clone());
        self.root.borrow_mut().children.push(child);
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

    pub fn resize(&mut self, width: u16, height: u16) {
        print!("{}", Clear(ClearType::All));
        self.viewport.screen = (width, height);
        let mut root = self.root.borrow_mut();
        root.calculate_canvas(Offset::default());
        root.render(self.viewport);
    }

    pub fn run(&mut self) -> io::Result<()> {
        self.prepare_screen()?;
        self.resize(self.viewport.screen.0, self.viewport.screen.1);

        loop {
            if event::poll(Duration::from_millis(100))? {
                use event::*;
                match event::read()? {
                    Event::Key(event) => match event.code {
                        KeyCode::Esc => {
                            return Ok(());
                        }
                        event => println!("{event:?}"),
                    },
                    Event::Mouse(event) => {
                        println!("{event:?}");
                    }
                    Event::Resize(width, height) => {
                        self.resize(width, height);
                        println!("Resize {width}x{height}")
                    }
                    event => println!("{event:?}"),
                }

                self.root.borrow().render(self.viewport);
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
