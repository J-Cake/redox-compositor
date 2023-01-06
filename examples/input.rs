use std::fs::OpenOptions;
use std::io::Read;
use std::{io, mem, slice};

#[derive(Copy, Clone, Debug)]
#[repr(packed)]
pub struct Event {
    pub code: i64,
    pub a: i64,
    pub b: i64,
}

impl Event {
    /// Create a null event
    pub fn new() -> Event {
        Event {
            code: 0,
            a: 0,
            b: 0,
        }
    }
}

unsafe fn read_to_slice<R: Read, T: Copy>(mut r: R, buf: &mut [T]) -> io::Result<usize> {
    r.read(slice::from_raw_parts_mut(
        buf.as_mut_ptr() as *mut u8,
        buf.len() * mem::size_of::<T>())
    ).map(|count| count / mem::size_of::<T>())
}

fn main() {
    let mut input = OpenOptions::new()
        .read(true)
        .write(true)
        .open("display:3/activate")
        .unwrap();

    loop {
        let mut buf = [Event::new(); 16];

        match unsafe { read_to_slice(&mut input, &mut buf) } {
            Ok(len) => {
                let packets = &mut buf[..len];

                for packet in packets {
                    println!("{:?}", packet);
                };
            }
            _ => {}
        }
    }
}
