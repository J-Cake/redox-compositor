use std::fs::{File, OpenOptions};
use std::{io, mem, slice};
use std::io::Write;
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use syscall::PAGE_SIZE;

#[derive(Clone, Copy)]
#[repr(packed)]
struct SyncRect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

fn main() {
    let mut file = syscall::open(std::env::args().skip(1).next().unwrap(), syscall::O_CLOEXEC | syscall::O_NONBLOCK | syscall::O_RDWR)
        .map(|socket| {
            unsafe { File::from_raw_fd(socket as RawFd) }
        }).unwrap();

    let mut buf = [0; 4096];
    let count = syscall::fpath(file.as_raw_fd() as usize, &mut buf).unwrap();
    let info = String::from_utf8_lossy(&buf[..count]).to_string();
    let info: Vec<_> = info.split('/').skip(1).take(2).map(|i| i.parse::<i32>().unwrap()).collect();
    let (width, height) = (info[0], info[1]);

    let mut surface = unsafe {
        let data = syscall::fmap(file.as_raw_fd() as usize, &syscall::Map {
            offset: 0,
            size: (width * height * 4) as usize ,//+ (PAGE_SIZE - 1) & !(PAGE_SIZE - 1),
            flags: syscall::PROT_READ | syscall::PROT_WRITE,
            address: 0,
        }).unwrap();
        let backing = std::slice::from_raw_parts_mut(data as *mut u32, (width * height) as usize);
        backing
        // raqote::DrawTarget::from_backing(width, height, backing)
    };

    surface.fill(0xffffffff);

    file.write(unsafe {
        slice::from_raw_parts(
            &(SyncRect {
                x: 0,
                y: 0,
                w: width,
                h: height
            }) as *const SyncRect as *const u8,
            mem::size_of::<SyncRect>()
        )
    }).unwrap();

    // surface.clear(raqote::SolidSource::from_unpremultiplied_argb(0xff, 0xff, 0xff, 0xff));

    // file.sync_all().unwrap();
    syscall::fsync(file.as_raw_fd() as usize).unwrap();
}
