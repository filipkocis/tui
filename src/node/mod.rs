mod handle;
pub mod utils;

pub use handle::{NodeHandle, WeakNodeHandle};

use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

use crate::{
    Canvas, Context, EventHandlers, HitMap, IntoEventHandler, Offset, Size, Style, Viewport,
    text::Text,
    workers::{WorkerFn, Workers},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Unique node id
pub struct NodeId(u64);

impl NodeId {
    /// Thread-safe unique id generator using atomic operations
    fn generate_id() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};

        static ID: AtomicU64 = AtomicU64::new(1);
        Self(ID.fetch_add(1, Ordering::Relaxed))
    }

    /// Creates a unique node id
    #[inline]
    pub fn new() -> Self {
        Self::generate_id()
    }

    /// Creates a node id from a u64
    #[inline]
    pub(crate) fn new_from(id: u64) -> Self {
        Self(id)
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

#[derive(Debug)]
pub struct Node {
    id: NodeId,
    pub name: String,

    pub class: String,
    pub style: Style,
    pub text: Text,

    /// Weak ref to parent. Use with caution to prevent deadlocks or memory leaks
    pub parent: Option<WeakNodeHandle>,
    /// Children of this node. Use with caution, when adding children, make sure to set the
    /// parent of the child node to this node's weak handle
    pub children: Vec<NodeHandle>,

    /// Event handlers registered on this node. It's not public since eventhandlers take `self` as
    /// an argument
    handlers: Rc<RefCell<EventHandlers>>,
    pub(crate) workers: Workers,

    // pub focus_within: bool,
    // pub hover_within: bool,
    // pub focused: bool,
    // pub hovered: bool,
    canvas: Canvas,
    /// Cached data for the node, used for optimizations. Do **NOT** mutate this directly.
    /// To get a reference use [`Node::cache`].
    cache: RefCell<NodeCache>,
}

impl Default for Node {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Default, Clone)]
/// Cached data for the node, used to optimize rendering, computation, and event handling.
pub struct NodeCache {
    /// Latest arg for [`Node::calculate_canvas`]
    pub parent_position: Offset,
    /// Latest arg for [`Node::calculate_percentage_size`]
    pub parent_available_size: Size,
    /// Latest style used for the node's canvas, at [`Node::calculate_canvas`]
    pub style: Style,
    /// Latest viewport used for rendering the node, at [`Node::render_to`]
    pub viewport: Viewport,
    /// Latest canvas position, computed at [`Node::calculate_canvas`]
    pub canvas_position: (i16, i16),
}

impl Node {
    /// Returns a default Node
    pub fn new() -> Self {
        let id = NodeId::default();

        Self {
            id,
            name: String::default(),
            class: String::default(),
            style: Style::default(),
            text: Text::default(),
            parent: Option::default(),
            children: Vec::default(),
            handlers: Rc::default(),
            workers: Workers::new(id),
            canvas: Canvas::default(),
            cache: RefCell::default(),
        }
    }

    /// Wraps the node in a [`NodeHandle`]
    pub fn into_handle(self) -> NodeHandle {
        NodeHandle::new(self)
    }

    /// Get the node's unique id
    #[inline]
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// Get a reference to the node's [`cache`](NodeCache).
    #[inline]
    pub fn cache(&self) -> Ref<NodeCache> {
        self.cache.borrow()
    }

    /// Get a mutable reference to the node's [`cache`](NodeCache).
    #[inline]
    pub(crate) fn cache_mut(&self) -> RefMut<NodeCache> {
        self.cache.borrow_mut()
    }

    /// Returns whether absolute position `X, Y` is within the node's canvas. Does not check it's
    /// children
    #[inline]
    pub fn hit_test(&self, x: i16, y: i16) -> bool {
        self.canvas.hit_test(x, y)
    }

    /// Returns the node's canvas position
    #[inline]
    pub fn absolute_position(&self) -> (i16, i16) {
        self.canvas.position
    }

    /// Returns the focus cursor position, which will either be the text's cursor position, or the
    /// content-start of the node's canvas.
    #[inline]
    pub fn focus_cursor_position(&self) -> (u16, u16) {
        let (px, py) = self.canvas.position;
        let xw = self.style.border.2 as u16 + self.style.padding.left;
        let xh = self.style.border.0 as u16 + self.style.padding.top;

        // Base cursor position
        let x = px as i32 + xw as i32;
        let y = py as i32 + xh as i32;

        if let Some((cx, cy)) = self.text.cursor {
            return ((x + cx as i32).max(0) as u16, (y + cy as i32).max(0) as u16);
        }

        // If the cursor is not set, return the base position
        (x.max(0) as u16, y.max(0) as u16)
    }

    /// Primitive calculation of `pos - node.canvas.position`, clamped to 0
    #[inline]
    pub fn relative_position(&self, x: i16, y: i16) -> (u16, u16) {
        let x = x - self.canvas.position.0;
        let y = y - self.canvas.position.1;

        (x.max(0) as u16, y.max(0) as u16)
    }

    /// Sort children by z-index, absolute children take precedence over relative children with the
    /// same z-index. Returns the result.
    #[inline]
    pub fn z_sort_children(&self) -> Vec<&NodeHandle> {
        let mut children = self.children.iter().collect::<Vec<_>>();

        children.sort_by(|a, b| {
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
        });

        children
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


    /// Adds a child node to the current node. The child node's parent will be set `parent`, make
    /// sure the `parent` is a weak handle to this node.
    #[inline]
    pub fn add_child(&mut self, child: NodeHandle, parent: WeakNodeHandle) {
        assert_eq!(
            Rc::strong_count(child.inner()),
            1,
            "Child node must not be shared, it must be unique to this parent"
        );

        assert_eq!(
            parent
                .upgrade()
                .map(|p| p.try_borrow().ok().map(|p| p.id()))
                .flatten(),
            None,
            "Weak parent can be borrowed, which means it is not `self` ({:?})",
            self.id(),
        );

        child.borrow_mut().parent = Some(parent);
        self.children.push(child);
    }

    /// Start a new worker thread
    #[inline]
    pub fn start_worker(&mut self, f: impl WorkerFn) {
        self.workers.start(f);
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
        self.cache_mut().parent_available_size = parent_available_size;

        // Calculate the size of this node
        self.style
            .compute_percentage_size(parent_available_size, &mut self.text);

        // Calculate the available content size of tis node
        let available_content_size = self.available_content_size();

        // Text size is used only for auto-size calculation
        let (text_width, text_height) = self.text.get_visual_size();

        // Either max_size or total_size depending on flex direction, for auto-size calculation
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
        self.style.compute_intrinsic_size(&self.text);

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
    /// - `parent_position` is the start of this node's canvas from the parent's perspective.
    pub fn calculate_canvas(&mut self, parent_position: Offset) {
        self.cache_mut().style = self.style.clone();
        self.cache_mut().parent_position = parent_position;

        let offset_position = parent_position.add(self.style.offset);
        let content_position = offset_position.add_tuple((
            self.style.padding.left as i16 + self.style.border.2 as i16,
            self.style.padding.top as i16 + self.style.border.0 as i16,
        ));

        let mut canvas = Canvas {
            position: offset_position.tuple(),
            buffer: vec![],
        };
        self.cache_mut().canvas_position = canvas.position;

        // Add text (before children)
        canvas.add_text(&self.text, self.style.size);

        // Start children after text (always flex-col)
        let y_after_text = {
            let height = canvas.height() as i16;
            let gap_row = self.style.gap.1 as i16 * (height > 0) as i16;
            height + gap_row
        };

        let mut extra_offset = (0, y_after_text);
        let mut include_gap = false;

        let (relative_children_count, relative_children_size) =
            self.justify_relative_children_data();

        // Calculate the free content size, content_size - children_sizes
        let gap_count = relative_children_count.saturating_sub(1) as i16;
        let free_content_size = if self.style.flex_row {
            let content_width = self.style.clamped_width();
            let free_width = (content_width as i32 - relative_children_size as i32) as i16;
            free_width - (gap_count * self.style.gap.0 as i16)
        } else {
            let content_height = self.style.clamped_height();
            let free_height = (content_height as i32 - relative_children_size as i32) as i16;
            free_height - (gap_count * self.style.gap.1 as i16)
        };

        // Add start offset for justify-content to extra_offset
        if self.style.justify.is_end() {
            if self.style.flex_row {
                extra_offset.0 = free_content_size;
            } else {
                extra_offset.1 = free_content_size;
            }
        } else if self.style.justify.is_center() {
            if self.style.flex_row {
                extra_offset.0 = free_content_size / 2;
            } else {
                extra_offset.1 = free_content_size / 2;
            }
        } else if self.style.justify.is_space_around() {
            let start_offset = (free_content_size / (relative_children_count + 1) as i16).max(0);
            if self.style.flex_row {
                extra_offset.0 += start_offset;
            } else {
                extra_offset.1 += start_offset;
            }
        }

        for (i, child) in self.children.iter().enumerate() {
            let mut child = child.borrow_mut();

            // Get extra offset for flex alignment
            let extra_align_offset = self
                .style
                .align
                .get_child_extra_offset(&self.style, &child.style);

            if child.style.offset.is_absolutely_relative() {
                // Use parent's 0,0 for absolutely relative children
                child.calculate_canvas(content_position);
            } else {
                let child_start_position = content_position
                    .add_tuple(extra_offset) // Add accumulated extra positioning offset
                    .add_tuple(extra_align_offset); // Add extra offset for flex alignment
                child.calculate_canvas(child_start_position);
            }

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

            // Increment the canvas offset with justify spaced logic
            if self.style.justify.is_spaced() {
                let adjust_count = if self.style.justify.is_space_around() {
                    1 // adds an extra start offset
                } else {
                    -1 // between two children
                };

                let offset = (free_content_size
                    / (relative_children_count as isize + adjust_count) as i16)
                    .max(0);
                if self.style.flex_row {
                    extra_offset.0 += offset;
                } else {
                    extra_offset.1 += offset;
                }
            }

            // Add the child canvas to this node's canvas
            canvas.extend_child(&child.canvas, &self.style, include_gap, i == 0);

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
    pub fn render_to(&self, mut viewport: Viewport, canvas: &mut Canvas, hitmap: &mut HitMap) {
        self.cache_mut().viewport = viewport;

        let is_absolute = self.style.offset.is_absolute();

        viewport.min = if is_absolute {
            // If the node is absolutely positioned, the min viewport is it's own position
            (
                self.canvas.position.0.max(0) as u16,
                self.canvas.position.1.max(0) as u16,
            )
        } else {
            // If the node is relatively positioned, the min viewport is either it's own position
            // or the parent viewport min, depending on which is greater. This is to prevent
            // canvas underflow
            (
                (self.canvas.position.0.max(0) as u16).max(viewport.min.0),
                (self.canvas.position.1.max(0) as u16).max(viewport.min.1),
            )
        };

        let total_width = self.style.total_width();
        let total_height = self.style.total_height();
        let max = (
            (self.canvas.position.0 + total_width as i16).max(0) as u16,
            (self.canvas.position.1 + total_height as i16).max(0) as u16,
        );

        let abs_max = if is_absolute {
            viewport.screen
        } else {
            viewport.max
        };

        viewport.max = (max.0.min(abs_max.0), max.1.min(abs_max.1));

        hitmap.add_target_area(self.id(), &viewport);
        self.canvas.render_to(&viewport, canvas);

        let screen_overflow = (
            max.0.saturating_sub(viewport.screen.0),
            max.1.saturating_sub(viewport.screen.1),
        );

        let viewport_span = (
            viewport.max.0.saturating_sub(viewport.min.0),
            viewport.max.1.saturating_sub(viewport.min.1),
        );
        let parent_overflow = (
            total_width.saturating_sub(viewport_span.0),
            total_height.saturating_sub(viewport_span.1),
        );

        let viewport_underflow = (
            (viewport.min.0 as i32 - self.canvas.position.0 as i32)
                .max(0)
                .min(u16::MAX as i32) as u16,
            (viewport.min.1 as i32 - self.canvas.position.1 as i32)
                .max(0)
                .min(u16::MAX as i32) as u16,
        );

        let overflow = (
            screen_overflow
                .0
                .max(parent_overflow.0)
                .saturating_sub(viewport_underflow.0),
            screen_overflow
                .1
                .max(parent_overflow.1)
                .saturating_sub(viewport_underflow.1),
        );

        viewport.max.0 = viewport.max.0.saturating_sub(
            (self.style.padding.right + self.style.border.3 as u16).saturating_sub(overflow.0),
        );
        viewport.max.1 = viewport.max.1.saturating_sub(
            (self.style.padding.bottom + self.style.border.1 as u16).saturating_sub(overflow.1),
        );

        let viewport_offset = (
            self.style.padding.left + self.style.border.2 as u16,
            self.style.padding.top + self.style.border.0 as u16,
        );

        viewport.min.0 += viewport_offset.0.saturating_sub(viewport_underflow.0);
        viewport.min.1 += viewport_offset.1.saturating_sub(viewport_underflow.1);

        for child in self.z_sort_children() {
            let child = child.borrow();
            child.render_to(viewport, canvas, hitmap);
        }
    }

    /// Handle a single event for this node. Returns wheter it should stop propagating
    pub fn handle_event(&mut self, ctx: &mut Context, is_capturing: bool) -> bool {
        self.handlers
            .clone()
            .borrow_mut()
            .handle(ctx, self, is_capturing)
    }

    #[inline]
    pub fn add_handler<F: IntoEventHandler>(&mut self, handler: F, is_capturing: bool) {
        self.handlers
            .borrow_mut()
            .add_handler(handler, is_capturing);
    }

    /// Builds a `path` from target to root node, returning true if the target was found.
    /// TODO: remove this, this is a temporary solution
    pub fn build_path_to_node(
        &self,
        id: NodeId,
        path: &mut Vec<(Rc<RefCell<Node>>, WeakNodeHandle)>,
    ) -> bool {
        if self.id() == id {
            return true;
        }

        for child in &self.children {
            if child.borrow().build_path_to_node(id, path) {
                path.push((child.inner().clone(), child.weak()));
                return true;
            }
        }

        false
    }

    /// Returns the data necessary to justify-content, which is the number of relative
    /// children and their cumulative width or height, depending on the flex direction.
    #[inline]
    fn justify_relative_children_data(&self) -> (usize, u16) {
        let mut width = 0u16;
        let mut height = 0u16;
        let mut count = 0;

        for child in &self.children {
            let child = child.borrow();
            if child.style.offset.is_absolute() {
                continue;
            }

            let (w, h) = child.style.total_size();
            width = width.saturating_add(w);
            height = height.saturating_add(h);
            count += 1;
        }

        if self.style.flex_row {
            (count, width)
        } else {
            (count, height)
        }
    }
}
