use std::collections::VecDeque;
use std::fmt::{Debug, Formatter};
use std::mem;
use std::rc::Rc;

use bitflags::bitflags;
use raqote::{DrawOptions, DrawTarget, Image};
use syscall::PAGE_SIZE;

use client::frame::*;
use crate::config::DisplayConfig;
use crate::desktop::{Desktop, DisplayConfiguration};

pub struct Frame {
    pub title: String,
    pub id: usize,
    pub pos: Rect,
    prev_pos: Rect,
    buffer: Vec<u32>,
    flags: FrameFlags,
    events: VecDeque<FrameEvents>,
    display_config: Rc<DisplayConfiguration<'static>>
}

impl Iterator for Frame {
    type Item = FrameEvents;

    fn next(&mut self) -> Option<Self::Item> {
        return self.events.pop_front();
    }
}

impl Frame {
    pub(crate) fn new(id: usize, title: String, width: u32, height: u32, flags: FrameFlags, config: Rc<DisplayConfiguration<'static>>) -> Self {
        let size = Rect { x: 0, y: 0, w: width, h: height };
        Self {
            title,
            id,
            pos: size.clone(),
            prev_pos: size,
            buffer: aligned_vec(width, height),
            flags,
            events: VecDeque::from([FrameEvents::Redraw()]),
            display_config: config
        }
    }

    pub(crate) fn image(&self) -> Image {
        Image {
            width: self.pos.w as i32,
            height: self.pos.h as i32,
            data: &self.buffer,
        }
    }

    pub(crate) fn mut_ptr(&mut self) -> (*mut u32, usize) {
        (self.buffer.as_mut_ptr(), self.buffer.len())
    }

    pub(crate) fn handle_request(&mut self, req: FrameRequests) -> Result<(), ()> {
        match req {
            FrameRequests::Fullscreen(fill) => if fill {
                self.position(self.display_config.fill_screen(&self.pos));
                self.handle_request(FrameRequests::ZLock(ZIndex::Front)).unwrap();
            } else { self.restore() }
            _ => todo!()
        };

        Ok(())
    }

    pub fn restore(&mut self) {
        let size = self.pos.clone();
        self.position(self.prev_pos);
        self.prev_pos = size;
    }

    pub fn position(&mut self, rect: Rect) {
        let mut buf = aligned_vec(rect.w.clone(), rect.h.clone());

        { // copy old buffer to new
            let mut ctx = DrawTarget::from_backing(rect.w.clone() as i32, rect.h.clone() as i32, &mut buf);
            ctx.draw_image_at(0., 0., &self.image(), &DrawOptions::default());
        }

        self.buffer = buf;
        self.pos = rect.clone();
        self.events.push_back(FrameEvents::Position(rect.clone()));
    }
}


fn aligned_vec(width: u32, height: u32) -> Vec<u32> {
    #[repr(C, align(4096))]
    #[derive(Clone)]
    struct Vector([u32; PAGE_SIZE / mem::size_of::<u32>()]);

    let pages = ((width * height) as usize * mem::size_of::<u32>()) / mem::size_of::<Vector>() + 1;
    let mut vec = vec![Vector([0xff000000; PAGE_SIZE / mem::size_of::<u32>()]); pages];

    let ptr = vec.as_mut_ptr() as *mut u32;
    let len = (width * height) as usize;// * mem::size_of::<u32>();
    mem::forget(vec);

    unsafe { Vec::from_raw_parts(ptr, len, len) }
}