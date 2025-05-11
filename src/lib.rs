mod canvas;
mod char;
mod line;
mod node;
mod offset;
mod style;
mod viewport;
mod handler;
mod elements;

pub use elements::*;
pub use canvas::Canvas;
pub use handler::{EventHandlers, IntoEventHandler};
pub use char::{Char, Code};
pub use line::Line;
pub use node::Node;
use node::NodeHandle;
pub use offset::Offset;
pub use style::Style;
pub use viewport::Viewport;

use std::{io, time::Duration};

use crossterm::{
    self,
    event::{self, DisableFocusChange, DisableMouseCapture, EnableFocusChange, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

pub struct App {
    raw: bool,
    alternate: bool,
    root: NodeHandle,

    canvas: Canvas,
    viewport: Viewport,
}

impl App {
    pub fn new(root: NodeHandle) -> Self {
        App {
            raw: true,
            alternate: true,
            root,
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

        self.root.borrow_mut().calculate_canvas(Offset::default());
        self.render()
    }

    pub fn render(&mut self) -> io::Result<()> {
        self.root
            .borrow()
            .render_to(self.viewport, &mut self.canvas);
        // print!("{}", cursor::Hide);
        self.canvas.render()?;
        Ok(())
    }

    pub fn run(&mut self) -> io::Result<()> {
        self.prepare_screen()?;
        self.resize(self.viewport.screen.0, self.viewport.screen.1)?;

        loop {
            let mut render = false;

            while event::poll(Duration::from_millis(0))? {
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
                        self.resize(width, height)?;
                        println!("Resize {width}x{height}")
                    }
                    event => println!("{event:?}"),
                }

                render = true;
            }

            if render {
                self.render()?;
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
