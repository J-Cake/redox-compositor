use std::{mem, slice};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::fd::AsRawFd;
use std::path::Path;

use euclid::{Size2D, UnknownUnit};
use raqote::{DrawTarget, IntPoint, IntRect, PathBuilder, SolidSource};

pub struct Display<'a> {
    pub surface: DrawTarget<&'a mut [u32]>,
    // surface: Vec<u32>,
    backing: File,

    // pub size: Size2D<i32, UnknownUnit>,
    pub pos: IntPoint,
}

#[derive(Clone, Copy)]
#[repr(packed)]
struct SyncRect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

impl<'a> Display<'a> {
    pub fn new(display: &str, pos: &IntPoint) -> Result<Display<'a>, String> {
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
            backing,
            surface,
        };
        display.sync();
        Ok(display)
    }

    fn sync(&mut self) {
        self.backing.write(unsafe {
            slice::from_raw_parts(
                &(SyncRect {
                    x: 0,
                    y: 0,
                    w: self.surface.width(),
                    h: self.surface.height(),
                }) as *const SyncRect as *const u8,
                mem::size_of::<SyncRect>())
        }).unwrap();
        syscall::fsync(self.backing.as_raw_fd() as usize).unwrap();
    }

    pub fn draw(&mut self, surface: &mut DrawTarget) {
        // DrawTarget::from_backing(self.size.width, self.size.height, &mut self.surface)
        //     .copy_surface(surface, IntRect::from_origin_and_size(self.pos, self.size), IntPoint::new(0, 0));
        let size = Size2D::new(self.surface.width(), self.surface.height());
        self.surface.copy_surface(surface, IntRect::from_origin_and_size(self.pos, size), IntPoint::new(0, 0));

        self.sync();
    }
}
