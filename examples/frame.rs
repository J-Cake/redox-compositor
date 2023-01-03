use std::fs::{File, OpenOptions};
use std::os::fd::AsRawFd;

use euclid::{Size2D, UnknownUnit};
use raqote::DrawTarget;
use syscall::PAGE_SIZE;

pub struct Frame<'a> {
    pub(crate) surface: DrawTarget<&'a mut [u32]>,
    backing: File,
    size: Size2D<i32, UnknownUnit>,
    draw: Box<dyn Fn(&mut DrawTarget<&mut [u32]>)>,
}

impl<'a> AsRawFd for Frame<'a> {
    fn as_raw_fd(&self) -> i32 {
        self.backing.as_raw_fd()
    }
}

impl<'a> Frame<'a> {
    pub fn new(title: &str, width: i32, height: i32) -> std::io::Result<Frame<'a>> {
        match OpenOptions::new()
            .read(true)
            .write(true)
            .open(format!("comp:title={}&min-size={},{}", title, width, height)) {
            Ok(win) => {
                let mut ctx = unsafe {
                    let ptr = syscall::fmap(win.as_raw_fd() as usize, &syscall::Map {
                        offset: 0,
                        size: ((width * height * 4) as usize + (PAGE_SIZE - 1)) & !(PAGE_SIZE - 1),
                        flags: syscall::PROT_READ | syscall::PROT_WRITE,
                        address: 0,
                    }).unwrap();

                    let buffer = std::slice::from_raw_parts_mut(ptr as *mut u32, (width * height) as usize);
                    DrawTarget::from_backing(width, height, buffer)
                };

                ctx.clear(raqote::SolidSource { r: 0xff, g: 0xff, b: 0xff, a: 0xff });
                syscall::fsync(win.as_raw_fd() as usize).unwrap();

                Ok(Frame {
                    surface: ctx,
                    backing: win,
                    size: Size2D::new(width, height),
                    draw: Box::new(|surface| {
                        surface.clear(raqote::SolidSource { r: 0xff, g: 0xff, b: 0xff, a: 0xff });
                    }),
                })
            }
            Err(err) => Err(err)
        }
    }

    fn sync(&mut self) {
        syscall::fsync(self.as_raw_fd() as usize).unwrap();
    }

    pub(crate) fn on_render<F>(&mut self, render: F) -> &mut Self where F: Fn(&mut DrawTarget<&mut [u32]>) -> () + 'static {
        self.draw = Box::new(render);
        self.sync();
        self
    }

    pub(crate) fn update(&mut self) {
        (self.draw)(&mut self.surface);
        self.sync();
    }
}
