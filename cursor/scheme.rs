use std::collections::VecDeque;
use std::{fs, mem, slice};
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use euclid::*;
use raqote::*;
use syscall::{Packet, SchemeMut};

use crate::cursor::Cursor;

const TAIL_LENGTH: usize = 4;

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

pub struct Display<'a> {
    backing: fs::File,
    surface: DrawTarget<&'a mut [u32]>,
    prev_cursor: VecDeque<Option<(SyncRect, Vec<u32>)>>,

    pub pos: Point2D<i32, UnknownUnit>,
}

impl<'a> Display<'a> {
    pub fn open(display: &str, pos: Point2D<i32, UnknownUnit>) -> Result<Self, String> {
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
            pos,
            backing,
            surface,

            prev_cursor: VecDeque::new(),
        };

        display.sync(None);
        Ok(display)
    }

    pub fn get_events(&mut self) -> Vec<InputEvent> {
        unsafe fn read_to_slice<R: Read, T: Copy>(mut r: R, buf: &mut [T]) -> std::io::Result<usize> {
            r.read(slice::from_raw_parts_mut(
                buf.as_mut_ptr() as *mut u8,
                buf.len() * mem::size_of::<T>())
            ).map(|count| count / mem::size_of::<T>())
        }

        let mut buf: [InputEvent; 64] = [InputEvent::new(); 64];
        let count = unsafe { read_to_slice(&mut self.backing, &mut buf).unwrap() };

        Vec::from(&buf[..count])
    }

    fn sync(&mut self, rect: Option<SyncRect>) {
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

    pub fn draw_cursor(&mut self, cursor: &Cursor) {
        for cursor in 0..self.prev_cursor.len() {
            if let Some((rect, data)) = self.prev_cursor.get(cursor).unwrap() {
                self.surface.draw_image_at(rect.x as f32, rect.y as f32, &Image {
                    width: rect.w as i32,
                    height: rect.h as i32,
                    data: &data,
                    // data: &vec![0xff000000u32; rect.w as usize * rect.h as usize],
                }, &raqote::DrawOptions::default());
            }
        }

        while self.prev_cursor.len() > TAIL_LENGTH {
            self.prev_cursor.pop_front();
        }

        let img = cursor.image();

        let pos = (cursor.get_pos() - self.pos.to_vector()).to_f32();

        let mut data = DrawTarget::new(img.width, img.height);
        data.clear(SolidSource::from_unpremultiplied_argb(0xff, 0, 0, 0));

        data.draw_image_at(-pos.x, -pos.y, &Image {
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

        self.surface.draw_image_at(pos.x, pos.y, &img, &raqote::DrawOptions::new());
        self.sync(Some(Box2D::from_origin_and_size(
            pos.to_i32(),
            Size2D::new(img.width as i32, img.height as i32),
        ).into()));

        for a in 0..self.prev_cursor.len() {
            if let Some((rect, _)) = self.prev_cursor.get(a).unwrap() {
                self.sync(Some(rect.clone()));
            }
        }
    }
}

pub struct Scheme<'a> {
    pub displays: Vec<Display<'a>>,
    pub scheme: fs::File,
    pub cursor: Cursor,
}

impl<'a> SchemeMut for Scheme<'a> {
    fn open(&mut self, path: &str, flags: usize, uid: u32, gid: u32) -> syscall::Result<usize> {
        let parts = path.split('/');
        let pos = parts.clone().take(1).last().unwrap().split('-').map(|s| s.parse::<i32>().unwrap_or(0)).collect::<Vec<i32>>();
        let path = parts.collect::<Vec<&str>>().join("/");

        match Display::open(&path, Point2D::new(pos[0], pos[1])) {
            Ok(display) => self.displays.push(display),
            Err(err) => return Err(syscall::Error::new(syscall::EINVAL))
        }

        Ok(self.displays.len() - 1)
    }

    fn read(&mut self, id: usize, buf: &mut [u8]) -> syscall::Result<usize> {
        let pos = self.cursor.get_pos();
        let str = format!("{}-{}", pos.x, pos.y);
        let bytes = str.as_bytes();
        let len = bytes.len().min(buf.len());
        buf[..len].copy_from_slice(&bytes[..len]);
        Ok(len)
    }

    fn write(&mut self, id: usize, buf: &[u8]) -> syscall::Result<usize> {
        let pos = String::from_utf8_lossy(buf);
        let pos = pos.split('-').map(|i| i.parse::<i32>().unwrap()).collect::<Vec<_>>();

        self.cursor.set_pos(Point2D::new(pos[0], pos[1]));

        Ok(buf.len())
    }

    fn fsync(&mut self, id: usize) -> syscall::Result<usize> {
        todo!()
    }

    fn close(&mut self, id: usize) -> syscall::Result<usize> {
        self.displays.remove(id);
        Ok(0)
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

impl<'a> Scheme<'a> {
    pub fn run(&mut self) {
        loop {
            let mut packet = Packet::default();
            if let Ok(len) = self.scheme.read(&mut packet) {
                if len > 0 {
                    self.handle(&mut packet);
                    self.scheme.write(&packet).unwrap();
                }
            }

            for i in self.displays.iter_mut().map(|i| i.get_events()).flatten() {
                match i.code {
                    11 => {
                        self.cursor.set_pos(Point2D::new(i.a as i32, i.b as i32));
                    },
                    _ => {}
                }
            }

            self.displays.iter_mut().for_each(|d| d.draw_cursor(&self.cursor));
        }
    }
}
