mod handle;

pub use handle::{NodeHandle, WeakNodeHandle};

use std::{cell::RefCell, rc::Rc};

use crossterm::event::Event;

use crate::{Canvas, EventHandlers, IntoEventHandler, Offset, Size, Style, Viewport};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Unique node id
pub struct NodeId(u64);

impl NodeId {
    /// Thread-safe unique id generator using atomic operations
    fn generate_id() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};

        static ID: AtomicU64 = AtomicU64::new(0);
        Self(ID.fetch_add(1, Ordering::Relaxed))
    }

    /// Creates a unique node id
    #[inline]
    pub fn new() -> Self {
        Self::generate_id()
    }

    /// Returns the id as a u64
    #[inline]
    pub fn get(&self) -> u64 {
        self.0
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::generate_id()
    }
}

#[derive(Debug, Default)]
pub struct Node {
    id: NodeId,
    pub name: String,

    pub class: String,
    pub style: Style,
    pub content: String,

    parent: Option<WeakNodeHandle>,
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

    /// Get the node's unique id
    #[inline]
    pub fn id(&self) -> NodeId {
        self.id
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

    /// Returns the available content size for this node, it's the content size minus any gaps
    #[inline]
    pub fn available_content_size(&self) -> Size {
        let (gap_column, gap_row) = self.style.gap;
        let gap_count = self
            .children
            .iter()
            .filter(|n| n.borrow().style.offset.is_translate())
            .count()
            .saturating_sub(1) as u16;
        let mut size = self.style.size;

        if self.style.flex_row && gap_column > 0 {
            let gaps = gap_column * gap_count;
            size.width = size
                .width
                .set_computed_size(size.width.computed_size().saturating_sub(gaps))
        } else if !self.style.flex_row && gap_row > 0 {
            let gaps = gap_row * gap_count;
            size.height = size
                .height
                .set_computed_size(size.height.computed_size().saturating_sub(gaps))
        }

        size
    }

    /// Computes the node's size and canvas. This should be called before
    /// [rendering](Self::render_to)
    pub fn compute(&mut self, parent_position: Offset, parent_available_size: Size) {
        self.calculate_auto_intrinsic_size();
        self.calculate_percentage_size(parent_available_size);
        self.calculate_canvas(parent_position);
    }

    /// Calculates the percentage size of the node, applies clamping, calculates text wrapping
    /// height change (for auto), and resizes auto-size with text and **non-percentage** children (percentages are
    /// not taken into account in this step of resizing).
    ///
    /// This is the second step of size calculation process. Percentage and clamping calculation
    /// have a top-down direction. Auto-size resizing has a bottom-up direcion.
    ///
    /// # Note
    /// Clamps the size
    pub fn calculate_percentage_size(&mut self, parent_available_size: Size) {
        // Calculate the size of this node
        let (text_width, text_height) = self
            .style
            .compute_percentage_size(parent_available_size, &self.content);

        // Calculate the available content size of tis node
        let available_content_size = self.available_content_size();

        // Either max_size or total_size depending on flex direction
        // let mut width = self.style.size.width.computed_size();
        // let mut height = self.style.size.height.computed_size();
        let mut width = text_width;
        let mut height = 0;

        let mut had_first_child = false;
        for child in self.children.iter() {
            let mut child = child.borrow_mut();
            child.calculate_percentage_size(available_content_size);

            // Skip absolute children
            if child.style.offset.is_absolute() {
                continue;
            }

            // Get total clamped child size, if not a percentage
            let child_width =
                child.style.total_width() * !self.style.size.width.is_percent() as u16;
            let child_height =
                child.style.total_height() * !self.style.size.height.is_percent() as u16;

            // Accumulate node's size with children of non-percentage size
            if self.style.flex_row {
                width += child_width + self.style.gap.0 * had_first_child as u16;
                height = height.max(child_height);
            } else {
                height += child_height + self.style.gap.1 * had_first_child as u16;
                width = width.max(child_width);
            }

            had_first_child = true;
        }

        // Add text height as flex-col, since text isn't part of the flexbox
        let has_text = text_height > 0;
        height += text_height + self.style.gap.1 * had_first_child as u16 * has_text as u16;

        // Apply the resized accumulated size if auto
        if self.style.size.width.is_auto() {
            self.style.size.width = self.style.size.width.set_computed_size(width);
        }
        if self.style.size.height.is_auto() {
            self.style.size.height = self.style.size.height.set_computed_size(height);
        }

        // Clamp the size
        self.style.size = self
            .style
            .size
            .clamp_computed_size(self.style.min_size, self.style.max_size);
    }

    /// Calculates the auto size and intrinsic size of the node. It's the first step of size
    /// calculation process. Auto is calculated bottom-up, intrinsic is calculated top-down.
    ///
    /// # Note
    /// Does not clamp the size
    pub fn calculate_auto_intrinsic_size(&mut self) {
        // Compute intrinsic size and text with auto size
        self.style.compute_intrinsic_size(&self.content);

        // Either max_size or total_size depending on flex direction
        let mut width = self.style.size.width.computed_size();
        let mut height = self.style.size.height.computed_size();

        let mut had_first_child = false;
        for child in self.children.iter() {
            let mut child = child.borrow_mut();
            child.calculate_auto_intrinsic_size();

            // Skip absolute children
            if child.style.offset.is_absolute() {
                continue;
            }

            // Get total unclamped child size
            let child_width = child.style.total_width_unclamped();
            let child_height = child.style.total_height_unclamped();

            // Accumulate node's size with children
            if self.style.flex_row {
                width += child_width + self.style.gap.0 * had_first_child as u16;
                height = height.max(child_height);
            } else {
                height += child_height + self.style.gap.1 * had_first_child as u16;
                width = width.max(child_width);
            }

            had_first_child = true;
        }

        // Apply the accumulated size if auto
        if self.style.size.width.is_auto() {
            self.style.size.width = self.style.size.width.set_computed_size(width);
        }
        if self.style.size.height.is_auto() {
            self.style.size.height = self.style.size.height.set_computed_size(height);
        }
    }

    /// Computes the node's canvas, combines them in a bottom-up direction. Should be called after
    /// finishing the size calculation process.
    ///
    /// - `parent_position` is the start of this node's canvas.
    /// - `parent_size` is the available parent's content size for this child to grow into.
    pub fn calculate_canvas(&mut self, parent_position: Offset) {
        // Apply z-index sort
        self.z_sort_children();

        let offset_position = parent_position.add(self.style.offset);
        let content_position = offset_position.add_tuple((
            self.style.padding.left as i16 + self.style.border.2 as i16,
            self.style.padding.top as i16 + self.style.border.0 as i16,
        ));

        let mut canvas = Canvas {
            position: offset_position.tuple(),
            buffer: vec![],
        };

        // Add text (before children)
        canvas.add_text(&self.content, self.style.size);

        // Start children after text (always flex-col)
        let y_after_text = {
            let height = canvas.height() as i16;
            let gap_row = self.style.gap.1 as i16 * (height > 0) as i16;
            height + gap_row
        };

        let mut extra_offset = (0, y_after_text);
        let mut include_gap = false;
        for (i, child) in self.children.iter().enumerate() {
            let mut child = child.borrow_mut();
            let child_start_position = content_position.add_tuple(extra_offset);
            child.calculate_canvas(child_start_position);

            // Skip absolute children
            if child.style.offset.is_absolute() {
                continue;
            }

            // Increment the canvas offset for the next child
            if self.style.flex_row {
                extra_offset.0 += child.canvas.width() as i16 + self.style.gap.0 as i16;
            } else {
                extra_offset.1 += child.canvas.height() as i16 + self.style.gap.1 as i16;
            }

            // Add the child canvas to this node's canvas
            canvas.extend_child(
                &child.canvas,
                &self.style,
                include_gap,
                self.style.flex_row && i == 0,
            );

            // Include gap after first child is seen
            include_gap = true;
        }

        // Normalize the canvas to a block based on the style
        canvas.normalize(&self.style);

        // Add block styling
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
            (self.canvas.position.0 + self.style.total_width() as i16).max(0) as u16,
            (self.canvas.position.1 + self.style.total_height() as i16).max(0) as u16,
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
            (self.style.padding.right + self.style.border.3 as u16).saturating_sub(overflow.0),
        );
        viewport.max.1 = viewport.max.1.saturating_sub(
            (self.style.padding.bottom + self.style.border.1 as u16).saturating_sub(overflow.1),
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
                parent.upgrade().map(|parent| {
                    parent.borrow_mut().bubble_event(event);
                });
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
