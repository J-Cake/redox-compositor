// use core::error::Source;
use std::{mem, slice};
use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;

use euclid::{Box2D, Point2D, Size2D, UnknownUnit};
use raqote::{DrawOptions, DrawTarget, IntPoint, IntRect, PathBuilder, Point, SolidSource, Source};

use crate::cursor::Cursor;

const TAIL_LENGTH: usize = 1;

pub struct Display<'a> {
    pub surface: DrawTarget<&'a mut [u32]>,
    sender: Sender<InputEvent>,
    backing: File,

    pub pos: IntPoint,

    prev_cursor: VecDeque<Option<(SyncRect, Vec<u32>)>>,
}

#[derive(Clone, Copy)]
#[repr(packed)]
pub(crate) struct SyncRect {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) w: i32,
    pub(crate) h: i32,
}

impl From<Box2D<i32, UnknownUnit>> for SyncRect {
    fn from(value: Box2D<i32, UnknownUnit>) -> Self {
        Self {
            x: value.min.x,
            y: value.min.y,
            w: value.max.x - value.min.x,
            h: value.max.y - value.min.y,
        }
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(packed)]
pub struct InputEvent {
    pub code: i64,
    pub a: i64,
    pub b: i64,
}

impl InputEvent {
    /// Create a null event
    pub fn new() -> InputEvent {
        InputEvent {
            code: 0,
            a: 0,
            b: 0,
        }
    }
}

unsafe fn read_to_slice<R: Read, T: Copy>(mut r: R, buf: &mut [T]) -> std::io::Result<usize> {
    r.read(slice::from_raw_parts_mut(
        buf.as_mut_ptr() as *mut u8,
        buf.len() * mem::size_of::<T>())
    ).map(|count| count / mem::size_of::<T>())
}

impl<'a> Display<'a> {
    pub fn new(display: &str, pos: &IntPoint, sender: Sender<InputEvent>) -> Result<Display<'a>, String> {
        let mut backing = match OpenOptions::new()
            .read(true)
            .write(true)
            .open(display) {
            Ok(file) => file,
            Err(_) => { return Err(format!("Unable to open display {}", display)); }
        };

        let (width, height) = {
            let mut buf: [u8; 4096] = [0; 4096];
            let count = syscall::fpath(backing.as_raw_fd() as usize, &mut buf).unwrap();

            println!("Opening {}", String::from_utf8_lossy(&buf[..count]));

            let url = match String::from_utf8(Vec::from(&buf[..count])) {
                Ok(url) => url,
                Err(_) => { return Err(format!("Received invalid information opening {}", display)); }
            };

            let mut url_parts = url.split(':');
            let _scheme_name = url_parts.next().unwrap();
            let path = url_parts.next().unwrap();

            let mut path_parts = path.split('/').skip(1);

            (path_parts.next().unwrap_or("").parse::<u32>().unwrap_or(0),
             path_parts.next().unwrap_or("").parse::<u32>().unwrap_or(0))
        };

        println!("Success - {}x{}", &width, &height);

        let mut surface = unsafe {
            let ptr = syscall::fmap(backing.as_raw_fd() as usize, &syscall::Map {
                offset: 0,
                size: (width * height * 4) as usize,
                flags: syscall::PROT_READ | syscall::PROT_WRITE,
                address: 0,
            }).unwrap();

            DrawTarget::from_backing(width as i32, height as i32, slice::from_raw_parts_mut(ptr as *mut u32, (width * height) as usize))
        };

        surface.clear(SolidSource { r: 0xff, g: 0xff, b: 0xff, a: 0xff });

        let mut display = Self {
            pos: pos.clone(),
            sender,
            backing,
            surface,

            prev_cursor: VecDeque::new(),
        };

        display.sync(None);
        Ok(display)
    }

    pub fn get_bounds(&self) -> Box2D<i32, UnknownUnit> {
        Box2D::new(
            self.pos,
            self.pos + Size2D::new(self.surface.width(), self.surface.height()),
        )
    }

    pub(crate) fn draw_cursor(&mut self, cursor: &Cursor) {
        for cursor in 0..self.prev_cursor.len() {
            if let Some((rect, data)) = self.prev_cursor.get(cursor).unwrap() {
                self.surface.draw_image_at(rect.x as f32, rect.y as f32, &raqote::Image {
                    width: rect.w as i32,
                    height: rect.h as i32,
                    data: &data,
                    // data: &vec![0xff000000u32; rect.w as usize * rect.h as usize],
                }, &DrawOptions::default());
            }
        }

        while self.prev_cursor.len() > TAIL_LENGTH {
            self.prev_cursor.pop_front();
        }

        let img = cursor.image();

        let pos = (cursor.get_pos() - self.pos.to_vector()).to_f32();

        let mut data = DrawTarget::new(img.width, img.height);
        data.clear(SolidSource::from_unpremultiplied_argb(0xff, 0, 0, 0));

        data.draw_image_at(-pos.x, -pos.y, &raqote::Image {
            width: self.surface.width(),
            height: self.surface.height(),
            data: self.surface.get_data(),
        }, &DrawOptions::default());

        self.prev_cursor.push_back(Some((SyncRect {
            x: pos.x as i32,
            y: pos.y as i32,
            w: img.width,
            h: img.height
        }, data.into_vec())));

        self.surface.draw_image_at(pos.x, pos.y, &img, &DrawOptions::new());
        self.sync(Some(IntRect::from_origin_and_size(
            pos.to_i32(),
            Size2D::new(img.width as i32, img.height as i32),
        ).into()));

        for a in 0..self.prev_cursor.len() {
            if let Some((rect, _)) = self.prev_cursor.get(a).unwrap() {
                self.sync(Some(rect.clone()));
            }
        }
    }

    pub(crate) fn sync(&mut self, rect: Option<SyncRect>) {
        if let Ok(event) = self.fetch_event() {
            for event in event {
                self.sender.send(event).unwrap();
            }
        }

        self.backing.write(unsafe {
            slice::from_raw_parts(
                &(rect.unwrap_or(SyncRect {
                    x: 0,
                    y: 0,
                    w: self.surface.width(),
                    h: self.surface.height(),
                })) as *const SyncRect as *const u8,
                mem::size_of::<SyncRect>())
        }).unwrap();
        syscall::fsync(self.backing.as_raw_fd() as usize).unwrap();
    }

    pub fn draw(&mut self, surface: &mut DrawTarget) {
        let size = Size2D::new(self.surface.width(), self.surface.height());
        self.surface.copy_surface(surface, IntRect::from_origin_and_size(self.pos, size), IntPoint::new(0, 0));
    }

    pub fn fetch_event(&mut self) -> std::io::Result<Vec<InputEvent>> {
        let mut buf: [InputEvent; 64] = [InputEvent::new(); 64];
        let count = unsafe { read_to_slice(&mut self.backing, &mut buf)? };

        Ok(Vec::from(&buf[..count]))
    }
}
