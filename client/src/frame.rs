use std::io::Read;

use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct FrameFlags: u64 {
        const FULLSCREEN =      0b1;
        const RESIZEX =        0b10;
        const RESIZEY =       0b100;
        const MOVEX =        0b1000;
        const MOVEY =       0b10000;
        const CLOSE =      0b100000;
    }
}

impl Default for FrameFlags {
    fn default() -> Self {
        Self::RESIZEX | Self::RESIZEY | Self::MOVEX | Self::MOVEY
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    pub fn zero() -> Self { Self { x: 0, y: 0, w: 0, h: 0 } }
}

bitflags! {
    #[derive(Debug, Clone)]
    #[repr(packed)]
    pub struct MouseButton: u8 {
        const LEFT = 0b100;
        const MIDDLE = 0b10;
        const RIGHT = 0b1;
    }
}

#[derive(Debug)]
pub enum Input {
    MouseMove(i32, i32),
    // which ever variants appear in the enum, are the buttons which are currently depressed. **all others** are not
    MouseButtonE(MouseButton),
    Scroll(f64, f64),
    // which ever key codes appear in the vec, are the keys which are currently depressed. **all others** are not
    Key(Vec<u16>),
}

// compositor -> client
#[derive(Debug)]
pub enum FrameEvents {
    // the frame was moved or resized
    Position(Rect),
    // visibility status changed
    Visible(bool),
    // the frame received an input event
    Input(Input),
    // the compositor requests the frame to redraw
    Redraw(),
    // the compositor requests the frame to close
    Close(),
    // the frame flags were changed, act accordingly
    Flags(FrameFlags),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ZIndex {
    Front,
    Automatic,
    Back,
}

// client -> compositor
#[derive(Debug, Copy, Clone)]
pub enum FrameRequests {
    // The client wishes to position the frame
    Position(Rect),
    // The client requests the fullscreen status be set to:
    Fullscreen(bool),
    // The client wishes to set the following flags
    Flags(FrameFlags),
    // The client wishes to set the minimised status
    Minimise(bool),
    // The client wishes to set a Z lock
    ZLock(ZIndex),
    // The client wishes to close the connection
    Close(),
}