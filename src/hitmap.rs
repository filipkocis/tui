use crate::{NodeId, Viewport};

#[derive(Debug, Default)]
pub struct HitMap {
    map: Vec<Vec<NodeId>>,
}

impl HitMap {
    pub fn new(width: usize, height: usize) -> Self {
        let mut map = Vec::with_capacity(height);

        for _ in 0..height {
            map.push(vec![NodeId::new_from(0); width]);
        }

        Self { map }
    }

    /// Returns the id of the cell at `x, y`
    pub fn get(&self, x: u16, y: u16) -> Option<NodeId> {
        self.map
            .get(y as usize)
            .and_then(|row| row.get(x as usize).copied())
    }

    /// Resizes the hitmap to the new size, resets all cells with `NodeId(0)`
    pub fn resize(&mut self, width: u16, height: u16) {
        let width = width as usize;
        let height = height as usize;

        for row in &mut self.map {
            for cell in row.iter_mut() {
                *cell = NodeId::new_from(0);
            }
        }

        self.map.resize(height, vec![NodeId::new_from(0); width]);

        for row in &mut self.map {
            row.resize(width, NodeId::new_from(0));
        }
    }

    /// Sets the target area of the hitmap to `id`
    pub fn add_target_area(&mut self, id: NodeId, viewport: &Viewport) {
        let (x, y) = viewport.min;
        let (width, height) = viewport.max;

        let self_height = self.map.len() as u16;
        let self_width = self.map.get(0).map_or(0, |row| row.len()) as u16;

        let height = height.min(self_height);
        let width = width.min(self_width);

        for i in y..height {
            for j in x..width {
                self.map[i as usize][j as usize] = id;
            }
        }
    }

    pub fn debug_render(&self) {
        use crossterm::cursor::MoveTo;
        use std::io::{Write, stdout};

        for (i, row) in self.map.iter().enumerate() {
            print!("{}", MoveTo(0, i as u16));
            for id in row {
                print!("{}", id.get() % 10);
            }
        }

        stdout().flush().unwrap()
    }
}
