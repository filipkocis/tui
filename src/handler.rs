use std::fmt::Debug;

use crossterm::event::Event;

use crate::Node;

pub type EventHandler = Box<dyn FnMut(&Event, &mut Node) -> bool>;

pub trait IntoEventHandler {
    fn into_event_handler(self) -> EventHandler;
}

impl<F> IntoEventHandler for F
where 
    F: FnMut(&Event, &mut Node) -> bool + 'static
{
    #[inline]
    fn into_event_handler(self) -> EventHandler {
        Box::new(self)
    }
}

#[derive(Default)]
pub struct EventHandlers {
    pub capturing: Vec<EventHandler>,
    pub bubbling: Vec<EventHandler>,
}

impl Debug for EventHandlers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventHandlers")
            .field("capturing", &self.capturing.len())
            .field("bubbling", &self.bubbling.len())
            .finish()
    }
}

impl EventHandlers {
    #[inline]
    pub fn add_handler<F: IntoEventHandler>(&mut self, handler: F, is_capturing: bool) {
        let handler = handler.into_event_handler();
        if is_capturing {
            self.capturing.push(handler)
        } else {
            self.bubbling.push(handler)
        }
    }

    pub fn handle(&mut self, node: &mut Node, event: &Event, is_capturing: bool) -> bool {
        let mut handled = false;

        let handlers = if is_capturing {
            &mut self.capturing
        } else {
            &mut self.bubbling
        };

        for handler in handlers {
            if handler(event, node) {
                handled = true;
            }
        }
        handled
    }
}
