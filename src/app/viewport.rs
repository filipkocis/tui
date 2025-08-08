use crossterm::terminal;

#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    pub min: (u16, u16),
    pub max: (u16, u16),
    pub screen: (u16, u16),
}

impl Viewport {
    /// Creates a new viewport with `max` and `screen` set to `(width, height)`
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            min: (0, 0),
            max: (width, height),
            screen: (width, height),
        }
    }

    /// Creates a new viewport with `max` and `screen` set to `(width, height)`
    #[inline]
    pub fn resize(&mut self, width: u16, height: u16) {
        self.screen = (width, height);
        self.max = (width, height);
    }

    pub fn debug_render(&self) {
        let mut hitmap = crate::HitMap::new(self.screen.0 as usize, self.screen.1 as usize);

        hitmap.add_target_area(crate::NodeId::new_from(1), self);
        hitmap.debug_render();
    }
}

impl Default for Viewport {
    /// A default viewport set to the terminal size
    fn default() -> Self {
        let (width, height) = terminal::size().unwrap();
        Self::new(width, height)
    }
}
