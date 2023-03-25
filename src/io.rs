use std::collections::VecDeque;

use euclid::{Point2D, Size2D, UnknownUnit};

use crate::frame::{FrameEvent, FrameRequest};

pub enum InputEvent {
    MouseMove(Point2D<i32, UnknownUnit>),
    MouseDown(Point2D<i32, UnknownUnit>, MouseButton),
    MouseUp(Point2D<i32, UnknownUnit>, MouseButton),
    MouseScroll(Point2D<i32, UnknownUnit>, i32),
    KeyDown(KeyCode),
    KeyUp(KeyCode),
}

pub enum Request<'a> {
    FrameRequest(FrameRequest),
}

pub enum Event<'a> {
    FrameEvent(FrameEvent<'a>),

    Input(InputEvent),
}

pub struct EventContainer<'a> {
    pub event: Event<'a>,
    pub msg_id: u128,
}

pub trait EventSource {
    fn poll(&mut self) -> Option<EventContainer>;
}

pub struct EventQueue<'a> {
    pub sources: Vec<Box<dyn EventSource>>,
}

impl EventQueue {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    pub fn run(&mut self) {
        loop {
            let mut queue = VecDeque::new();

            for i in &mut self.sources {
                while let Some(event) = i.poll() {
                    queue.push_back(event);
                }
            }

            for i in queue.drain(..) {
                self.handle_event(&i);
            }
        }
    }

    pub fn handle_event(&self, event: &EventContainer) {
        match &event.event {
            _ => todo!() // Delegate events to the correct handler
        }
    }
}
