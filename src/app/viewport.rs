use crossterm::terminal;

#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    pub min: (u16, u16),
    pub max: (u16, u16),
    pub screen: (u16, u16),
}

impl Viewport {
    pub fn new() -> Self {
        let (width, height) = terminal::size().unwrap();
        Self {
            min: (0, 0),
            max: (width, height),
            screen: (width, height),
        }
    }

    pub fn debug_render(&self) {
        let mut hitmap = crate::HitMap::new(self.screen.0 as usize, self.screen.1 as usize);

        hitmap.add_target_area(crate::NodeId::new_from(1), self);
        hitmap.debug_render();
    }
}

impl Default for Viewport {
    fn default() -> Self {
        Self::new()
    }
}
