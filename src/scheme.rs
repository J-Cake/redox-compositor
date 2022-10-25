use std::{mem, slice};
use std::num::ParseIntError;

use raqote::DrawOptions;
use syscall::{PAGE_SIZE, SchemeMut};

use client::frame::*;
use client::serialise::Serialise;

use crate::desktop::Desktop;
use crate::display::Display;
use crate::frame::Frame;

pub fn parse(split: &mut core::str::Split<&str>) -> Result<(u32, u32), syscall::Error> {
    let mut size = (0, 0);

    match split.next().unwrap_or("").parse::<u32>() {
        Ok(dimen) => size.0 = dimen,
        Err(_) => { return Err(syscall::Error::new(syscall::EINVAL)); }
    };
    match split.next().unwrap_or("").parse::<u32>() {
        Ok(dimen) => size.1 = dimen,
        Err(_) => { return Err(syscall::Error::new(syscall::EINVAL)); }
    };

    Ok(size)
}

// impl SchemeMut for Desktop {
//     fn open(&mut self, specifier: &str, _flags: usize, _uid: u32, _gid: u32) -> syscall::Result<usize> {
//         let mut split = specifier.split(";");
//
//         let id = self.next_id();
//         let title = split.next().unwrap_or("");
//
//         let size = match parse(&mut split) {
//             Ok(size) => size,
//             Err(err) => { return Err(err); }
//         };
//
//         let frame = self.push_frame(Frame::new(id, title, size.0, size.1));
//
//         Ok(frame.id)
//     }
//
//     fn unlink(&mut self, path: &str, uid: u32, gid: u32) -> syscall::Result<usize> {
//         eprintln!("Warning: Use of `unlink`");
//
//         let mut split = path.split(";");
//
//         let id = match split
//             .next()
//             .unwrap_or("")
//             .parse::<usize>() {
//             Ok(id) => id,
//             Err(_) => { return Err(syscall::Error::new(syscall::EINVAL)); }
//         };
//
//         return if uid == 0 || gid == 0 {
//             return match self.frames.remove(&id) {
//                 Some(_) => Ok(0),
//                 None => Err(syscall::Error::new(syscall::ENOENT))
//             };
//         } else {
//             Err(syscall::Error::new(syscall::EPERM))
//         };
//     }
//
//     fn read(&mut self, _file: usize, _buf: &mut [u8]) -> syscall::Result<usize> {
//         Ok(0)
//     }
//
//     fn write(&mut self, _file: usize, _buffer: &[u8]) -> syscall::Result<usize> {
//         Ok(0)
//     }
//
//     fn fmap_old(&mut self, id: usize, map: &syscall::OldMap) -> syscall::Result<usize> {
//         self.fmap(id, &syscall::Map {
//             offset: map.offset,
//             size: map.size,
//             flags: map.flags,
//             address: 0,
//         })
//     }
//
//     /// Map the buffer behind a frame into a client-side buffer.
//     ///
//     /// [I have no fucking clue](https://gitlab.redox-os.org/redox-os/orbital/-/blob/master/orbital-core/src/lib.rs)
//     fn fmap(&mut self, id: usize, map: &syscall::Map) -> syscall::Result<usize> {
//         match self.frames.get_mut(&id) {
//             Some(frame) => {
//                 let page_size = PAGE_SIZE;
//                 let map_pages = (map.offset + map.size + page_size - 1) / page_size;
//                 let (data_addr, len) = frame.mut_ptr();
//
//                 if map_pages * page_size <= len * mem::size_of::<u32>() {
//                     Ok((data_addr as usize) + map.offset)
//                 } else {
//                     Err(syscall::Error::new(syscall::EINVAL))
//                 }
//             }
//             _ => Err(syscall::Error::new(syscall::ENOENT))
//         }
//     }
//
//
//     /// Get the absolute path of the window - used in this case to fetch basic information about the window
//     fn fpath(&mut self, id: usize, buf: &mut [u8]) -> syscall::Result<usize> {
//         match self.frames.get(&id) {
//             Some(frame) => {
//                 let data = format!("Frame:{}-{}x{}", &frame.id, frame.size().w, frame.size().h);
//                 for (a, i) in data.bytes().enumerate() {
//                     buf[a] = i;
//                 }
//
//                 Ok(data.len())
//             }
//             None => Err(syscall::Error::new(syscall::ENOENT))
//         }
//     }
//
//     fn fsync(&mut self, id: usize) -> syscall::Result<usize> {
//         let frame = match self.frames.get(&id) {
//             Some(frame) => frame,
//             None => { return Err(syscall::Error::new(syscall::ENOENT)); }
//         };
//
//         self.ctx.draw_image_at(frame.size().x as f32, frame.size().y as f32, &frame.image(), &DrawOptions::default());
//
//         Ok(0)
//     }
//
//     fn close(&mut self, frame: usize) -> syscall::Result<usize> {
//         return match self.frames.remove(&frame) {
//             Some(frame) => {
//                 self.unused_frame_ids.push_back(frame.id);
//                 self.frame_order = self.frame_order.iter().filter(|i| self.frames.contains_key(*i)).map(|i| *i).collect();
//                 Ok(0)
//             }
//             None => Err(syscall::Error::new(syscall::ENOENT))
//         };
//     }
//
//     fn fchmod(&mut self, frame_id: usize, mode: u16) -> syscall::Result<usize> {
//         let mode = match FrameFlags::try_from(mode) {
//             Ok(mode) => mode,
//             Err(_) => { return Err(syscall::Error::new(syscall::EINVAL)); }
//         };
//
//         let frame = match self.frames.get(&frame_id) {
//             Some(frame) => frame,
//             None => { return Err(syscall::Error::new(syscall::ENOENT)); }
//         };
//
//         match mode {
//             FrameFlags::Fullscreen => {
//                 let size = self.fill_screen(frame.size());
//                 match self.frames.get_mut(&frame_id) {
//                     Some(frame) => frame.resize(size),
//                     None => { return Err(syscall::Error::new(syscall::ENOENT)); }
//                 };
//             },
//             _ => { return Err(syscall::Error::new(syscall::EINVAL)); }
//         }
//         Ok(0)
//     }
// }

impl<'a> SchemeMut for Desktop<'a> {
    fn open(&mut self, path: &str, flags: usize, uid: u32, gid: u32) -> syscall::Result<usize> {
        let mut path = path.split(";");
        let title = client::hex::dehex(path.next().unwrap_or("")).unwrap();
        let size = if let Ok(size) = parse(&mut path) { size } else { return Err(syscall::Error::new(syscall::EINVAL)); };

        let id = self.next_id();
        let bitflags = if let Some(bitflags) = FrameFlags::from_bits(flags as u64) { bitflags } else { return Err(syscall::Error::new(syscall::EINVAL)); };
        let frame = self.push_frame(Frame::new(id, title, size.0, size.1, bitflags, self.get_display_config()));

        Ok(frame.id)
    }

    fn read(&mut self, id: usize, buf: &mut [u8]) -> syscall::Result<usize> {
        let frame = if let Some(frame) = self.frames.get_mut(&id) { frame } else { return Err(syscall::Error::new(syscall::EINVAL)); };

        if let Some(mut event) = frame.next() {
            if buf.len() > 0 {
                let bytes: Vec<u8> = event.to_bytes().unwrap();
                for (a, i) in bytes.iter().enumerate() {
                    buf[a] = *i;
                }
                Ok(bytes.len())
            } else {
                Err(syscall::Error::new(syscall::ENOSPC))
            }
        } else {
            Err(syscall::Error::new(syscall::EAGAIN))
        }
    }

    fn write(&mut self, id: usize, buf: &[u8]) -> syscall::Result<usize> {
        let Some(frame) = self.frames.get_mut(&id) else { return Err(syscall::Error::new(syscall::EINVAL)); };

        let req = match FrameRequests::from_bytes(buf) {
            Ok(req) => req,
            Err(err) => { return Err(syscall::Error::new(syscall::EINVAL)); }
        };

        match frame.handle_request(req) {
            Ok(..) => Ok(mem::size_of_val(&req)),
            Err(..) => Err(syscall::Error::new(syscall::EINVAL))
        }
    }

    fn fmap_old(&mut self, id: usize, map: &syscall::OldMap) -> syscall::Result<usize> {
        self.fmap(id, &syscall::Map {
            offset: map.offset,
            size: map.size,
            flags: map.flags,
            address: 0,
        })
    }

    fn fmap(&mut self, id: usize, map: &syscall::Map) -> syscall::Result<usize> {
        if let Some(frame) = self.frames.get_mut(&id) {
            let map_pages = (map.offset + map.size + (PAGE_SIZE - 1)) / PAGE_SIZE;
            let (data_addr, len) = frame.mut_ptr();

            if map_pages * PAGE_SIZE >= len * mem::size_of::<u32>() {
                Ok((data_addr as usize) + map.offset)
            } else {
                Err(syscall::Error::new(syscall::EINVAL))
            }
        } else {
            Err(syscall::Error::new(syscall::ENOENT))
        }
    }

    fn fsync(&mut self, id: usize) -> syscall::Result<usize> {
        let frame = if let Some(frame) = self.frames.get_mut(&id) { frame } else { return Err(syscall::Error::new(syscall::EINVAL)); };

        let Rect { x, y, .. } = frame.pos;
        self.ctx.draw_image_at(x as f32, y as f32, &frame.image(), &DrawOptions::default());

        self.flush();

        Ok(0)
    }

    fn close(&mut self, id: usize) -> syscall::Result<usize> {
        self.frames.remove(&id);
        self.unused_frame_ids.push_back(id);

        self.flush();

        Ok(0)
    }
}