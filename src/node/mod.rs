mod handle;

pub use handle::NodeHandle;

use std::{cell::RefCell, rc::Rc};

use crossterm::event::Event;

use crate::{Canvas, EventHandlers, IntoEventHandler, Offset, Size, Style, Viewport};

#[derive(Debug, Default)]
pub struct Node {
    pub id: String,
    pub class: String,
    pub style: Style,
    pub content: String,

    parent: Option<NodeHandle>,
    children: Vec<NodeHandle>,

    /// Event handlers registered on this node. It's not public since eventhandlers take `self` as
    /// an argument
    handlers: Rc<RefCell<EventHandlers>>,

    // pub focus_within: bool,
    // pub hover_within: bool,
    // pub focused: bool,
    // pub hovered: bool,
    canvas: Canvas,
}

impl Node {
    /// Wraps the node in a [`NodeHandle`]
    pub fn into_handle(self) -> NodeHandle {
        NodeHandle::new(self)
    }

    /// Returns the `total width - content` leaving horizontal padding and borders
    #[inline]
    pub fn extra_width(&self) -> u16 {
        self.style
            .padding
            .2
            .saturating_add(self.style.padding.3)
            .saturating_add(self.style.border.2 as u16)
            .saturating_add(self.style.border.3 as u16)
    }

    /// Returns the `total height - content` leaving vertical padding and borders
    #[inline]
    pub fn extra_height(&self) -> u16 {
        self.style
            .padding
            .0
            .saturating_add(self.style.padding.1)
            .saturating_add(self.style.border.0 as u16)
            .saturating_add(self.style.border.1 as u16)
    }

    /// Total computed width of the node
    pub fn total_width(&self) -> u16 {
        self.style
            .clamped_width()
            .saturating_add(self.extra_width())
    }

    /// Total computed height of the node
    pub fn total_height(&self) -> u16 {
        self.style
            .clamped_height()
            .saturating_add(self.extra_height())
    }

    /// Returns whether absolute position `X, Y` is within the node's canvas. Does not check it's
    /// children
    #[inline]
    pub fn hit_test(&self, x: i16, y: i16) -> bool {
        self.canvas.hit_test(x, y)
    }

    /// Primitive calculation of `pos - node.canvas.position`, clamped to 0
    #[inline]
    pub fn relative_position(&self, x: i16, y: i16) -> (u16, u16) {
        let x = x - self.canvas.position.0;
        let y = y - self.canvas.position.1;

        (x.max(0) as u16, y.max(0) as u16)
    }

    /// Sort children by z-index
    #[inline]
    pub fn z_sort_children(&mut self) {
        self.children.sort_by(|a, b| {
            let a = a.borrow();
            let b = b.borrow();

            if a.style.z_index == b.style.z_index {
                a.style
                    .offset
                    .is_absolute()
                    .cmp(&b.style.offset.is_absolute())
            } else {
                a.style.z_index.cmp(&b.style.z_index)
            }
        })
    }

    /// Computes the node's canvas. This should be called before [rendering](Self::render_to)
    pub fn calculate_canvas(&mut self, parent_position: Offset, parent_size: Size) {
        self.z_sort_children();

        let position = parent_position.add(self.style.offset);
        let content_position = position.add_tuple((
            self.style.padding.2 as i16 + self.style.border.2 as i16,
            self.style.padding.0 as i16 + self.style.border.0 as i16,
        ));

        self.style.compute_size_td(parent_size);

        let mut canvas = Canvas {
            position: position.tuple(),
            buffer: vec![],
        };

        let mut extra_offset = (0, 0);
        let mut include_gap = false;
        for (i, child) in self.children.iter().enumerate() {
            let mut child = child.borrow_mut();
            child.calculate_canvas(content_position.add_tuple(extra_offset), self.style.size);

            if child.style.offset.is_absolute() {
                continue;
            }

            if self.style.flex_row {
                extra_offset.0 += child.canvas.width() as i16 + self.style.gap.0 as i16;
            } else {
                extra_offset.1 += child.canvas.height() as i16 + self.style.gap.1 as i16;
            }

            canvas.extend_child(
                &child.canvas,
                &self.style,
                include_gap,
                self.style.flex_row && i == 0,
            );

            include_gap = true;
        }
        canvas.add_text(&self.content, self.style.size);
        canvas.normalize(&self.style);

        canvas.add_padding(self.style.padding);
        canvas.add_fg(self.style.fg);
        canvas.add_bg(self.style.bg);
        canvas.add_border(self.style.border);

        self.canvas = canvas;
    }

    /// Render the node and its children to `canvas` within the given `viewport`. Node's canvas has to be
    /// computed before calling this function.
    pub fn render_to(&self, mut viewport: Viewport, canvas: &mut Canvas) {
        viewport.min = (
            self.canvas.position.0.max(0) as u16,
            self.canvas.position.1.max(0) as u16,
        );

        let max = (
            (self.canvas.position.0 + self.total_width() as i16).max(0) as u16,
            (self.canvas.position.1 + self.total_height() as i16).max(0) as u16,
        );

        let abs_max = if self.style.offset.is_absolute() {
            viewport.screen
        } else {
            viewport.max
        };

        viewport.max = (max.0.min(abs_max.0), max.1.min(abs_max.1));

        self.canvas.render_to(&viewport, canvas);

        let overflow = (
            max.0.saturating_sub(viewport.screen.0),
            max.1.saturating_sub(viewport.screen.1),
        );

        viewport.max.0 = viewport.max.0.saturating_sub(
            (self.style.padding.3 + self.style.border.3 as u16).saturating_sub(overflow.0),
        );
        viewport.max.1 = viewport.max.1.saturating_sub(
            (self.style.padding.1 + self.style.border.1 as u16).saturating_sub(overflow.1),
        );

        for child in &self.children {
            let child = child.borrow();
            child.render_to(viewport, canvas);
        }
    }

    /// Propagate event down to children.
    pub fn propagate_event(&mut self, event: &Event) {
        let handled = self.handlers.clone().borrow_mut().handle(self, event, true);

        if !handled {
            for child in &self.children {
                let mut child = child.borrow_mut();
                child.propagate_event(event);
            }
        }
    }

    /// Bubble event up to the root node.
    pub fn bubble_event(&mut self, event: &Event) {
        let handled = self
            .handlers
            .clone()
            .borrow_mut()
            .handle(self, event, false);

        if !handled {
            if let Some(ref parent) = self.parent {
                parent.borrow_mut().bubble_event(event);
            }
        }
    }

    #[inline]
    pub fn add_handler<F: IntoEventHandler>(&mut self, handler: F, is_capturing: bool) {
        self.handlers
            .borrow_mut()
            .add_handler(handler, is_capturing);
    }
}
