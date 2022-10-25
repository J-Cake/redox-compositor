use std::fs::File;
use std::{io, slice};
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use raqote::DrawTarget;
use syscall::{O_CLOEXEC, O_NONBLOCK, O_RDWR};
use client::frame::Rect;

use crate::desktop::{Desktop};

pub struct Display {
    pub area: Rect,
    pub(crate) name: String,
    pub(crate) ctx: DrawTarget<&'static mut [u32]>,
    // represent the framebuffers of individual displays
    pub(crate) buffer: File,
}

impl<'a> Desktop<'a> {
    pub(crate) fn load_display(path: &str) -> io::Result<Display> {
        let display = syscall::open(&path, O_CLOEXEC | O_NONBLOCK | O_RDWR)
            .map(|socket| unsafe { File::from_raw_fd(socket as RawFd) })
            .map_err(|err| {
                eprintln!("orbital: failed to open display {}: {}", path, err);
                io::Error::from_raw_os_error(err.errno)
            })
            .expect("Failed to open file");

        let (width, height) = {
            let mut buf: [u8; 4096] = [0; 4096];
            let count = syscall::fpath(display.as_raw_fd() as usize, &mut buf).unwrap();

            let url = unsafe { String::from_utf8_unchecked(Vec::from(&buf[..count])) };

            let mut url_parts = url.split(':');
            let _scheme_name = url_parts.next().unwrap();
            let path = url_parts.next().unwrap();

            let mut path_parts = path.split('/').skip(1);

            (path_parts.next().unwrap_or("").parse::<u32>().unwrap_or(0),
             path_parts.next().unwrap_or("").parse::<u32>().unwrap_or(0))
        };

        Ok(Display {
            name: path.to_owned(),
            area: Rect {
                x: 0, y: 0, // TODO: read positions from config
                w: width,
                h: height
            },
            ctx: unsafe {
                let ptr = syscall::fmap(display.as_raw_fd() as usize, &syscall::Map {
                    offset: 0,
                    size: (width * height * 4) as usize,
                    flags: syscall::PROT_READ | syscall::PROT_WRITE,
                    address: 0,
                }).unwrap();

                let buffer = slice::from_raw_parts_mut(ptr as *mut u32, (width * height) as usize);
                DrawTarget::from_backing(width as i32, height as i32, buffer)
            },
            buffer: display,
        })
    }
}