pub mod node;

use std::{io, time::Duration};

use node::Node;

use crossterm::{
    self,
    event::{self, DisableFocusChange, DisableMouseCapture, EnableFocusChange, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

pub struct App {
    raw: bool,
    alternate: bool,
    root: Node,
}

impl App {
    pub fn new(root: Node) -> io::Result<App> {
        execute!(
            io::stdout(),
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableFocusChange
        )?;
        enable_raw_mode()?;

        Ok(App {
            raw: true,
            alternate: true,
            root,
        })
    }

    pub fn run(&mut self) -> io::Result<()> {
        self.root.render();

        loop {
            if event::poll(Duration::from_millis(100))? {
                if let event::Event::Key(key) = event::read()? {
                    match key.code {
                        event::KeyCode::Esc => {
                            return Ok(());
                        }
                        _ => {}
                    }
                }

                self.root.render();
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
