use std::{mem, thread};
use std::fs::OpenOptions;
use std::os::fd::AsRawFd;
use std::time::Duration;

use raqote::Color;
use syscall::PAGE_SIZE;

fn main() {
    let win = OpenOptions::new()
        .read(true)
        .write(true)
        .open("comp:title=Client&min-size=200,160")
        .expect("Unable to create window");

    let mut ctx = unsafe {
        let ptr = syscall::fmap(win.as_raw_fd() as usize, &syscall::Map {
            offset: 0,
            size: 200 * 160 * 4,
            flags: syscall::PROT_READ | syscall::PROT_WRITE,
            address: 0,
        }).unwrap();

        let buffer = std::slice::from_raw_parts_mut(ptr as *mut u32, 200 * 160);
        raqote::DrawTarget::from_backing(200, 160, buffer)
    };

    ctx.clear(raqote::SolidSource { r: 0xff, g: 0xff, b: 0xff, a: 0xff });

    syscall::fsync(win.as_raw_fd() as usize).unwrap();

    loop {
        ctx.clear(raqote::SolidSource::from_unpremultiplied_argb(0xff, 0xff, 0x00, 0xff));
        syscall::fsync(win.as_raw_fd() as usize).unwrap();
        thread::sleep(Duration::from_millis(16000));
    }
}
