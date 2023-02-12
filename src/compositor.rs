use std::borrow::{Borrow, BorrowMut};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::{Read, Write};
use std::mem::MaybeUninit;
use std::os::fd::{FromRawFd, RawFd};
use std::rc::Rc;
use std::sync::{Arc, Condvar, mpsc, Mutex};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use euclid::{Box2D, Point2D, SideOffsets2D, Size2D, UnknownUnit};
use lazy_static::lazy_static;
use raqote::{DrawOptions, DrawTarget, IntPoint, IntRect, SolidSource, Source};
use raqote::Source::Solid;
use syscall::{Event, Map, O_NONBLOCK, Packet, SchemeMut};

use crate::config::Config;
use crate::cursor::Cursor;
use crate::display::{Display, InputEvent, SyncRect};
use crate::frame::{Frame, FrameEvent, FrameOptions};
use crate::plugin;
use crate::plugin::{PluginEvent, PluginManager};

pub struct Compositor<'a, 'b> where 'b: 'a {
    pub displays: Vec<Display<'a>>,
    pub frames: HashMap<usize, Frame<'b>>,
    pub surface: DrawTarget,
    pub cursor: Cursor,
    pub scheme: File,

    last_update: Instant,
    events: Rc<Mutex<VecDeque<PluginEvent>>>,
    input: Receiver<InputEvent>,
}

pub const SCHEME_NAME: &'static str = ":comp";
pub const MAX_FPS: Duration = Duration::from_nanos(1_000_000_000 / 60);

impl<'a, 'b> Compositor<'a, 'b> {
    pub fn new(config: Config) -> Result<(Self, Rc<Mutex<VecDeque<PluginEvent>>>), String> {
        let (sender, receiver) = channel();
        let mut displays: Vec<Display> = config.displays.iter()
            .map(|(name, pos)| Display::new(&name, &pos, sender.clone())
                .expect("Failed to create display"))
            .collect();

        println!("Created {} displays", displays.len());

        let mut min = (0, 0);
        let mut max = (0, 0);

        displays.iter().for_each(|i| {
            min = (i.pos.x.min(min.0), i.pos.y.min(min.1));
            max = ((i.pos.x + i.surface.width()).max(max.0), (i.pos.y + i.surface.height()).max(max.1));
        });

        let events = Rc::new(Mutex::new(VecDeque::new()));

        let mut surface = DrawTarget::new(max.0 - min.0, max.1 - min.1);
        surface.clear(SolidSource::from_unpremultiplied_argb(0xff, 0, 0, 0));

        displays.iter_mut().for_each(|i| i.draw(&mut surface));
        displays.iter_mut().for_each(|i| i.sync(None));

        Ok((Compositor {
            last_update: Instant::now() - MAX_FPS,
            events: Rc::clone(&events),
            input: receiver,
            displays,
            frames: HashMap::new(),
            surface,//: DrawTarget::new(max.0 - min.0, max.1 - min.1),
            cursor: Cursor::new(min.0, max.0, min.1, max.1),
            scheme: syscall::open(SCHEME_NAME, syscall::O_CREAT | syscall::O_RDWR | syscall::O_CLOEXEC | O_NONBLOCK)
                .map(|socket| unsafe { File::from_raw_fd(socket as RawFd) })
                .unwrap_or_else(|_| {
                    syscall::open(SCHEME_NAME, syscall::O_RDWR | syscall::O_CLOEXEC | O_NONBLOCK)
                        .map(|socket| unsafe { File::from_raw_fd(socket as RawFd) })
                        .unwrap()
                }),
        }, Rc::clone(&events)))
    }

    pub fn tick(&mut self) {
        let mut packet = Packet::default();
        if let Ok(len) = self.scheme.read(&mut packet) {
            if len > 0 {
                self.handle(&mut packet);
                self.scheme.write(&packet).unwrap();
            }
        }

        while let Ok(event) = self.input.try_recv() {
            match event.code {
                11 => {
                    let (a, b) = (event.a, event.b);

                    self.cursor.move_cursor(a as i32, b as i32);
                    self.displays.iter_mut().for_each(|i| i.draw_cursor(&self.cursor));
                }
                _ => ()
            }
        }

        self.last_update = Instant::now();
    }

    pub fn to_local_mut(&mut self, pos: Point2D<i32, UnknownUnit>) -> Option<(&'a mut Display, Box2D<i32, UnknownUnit>, Point2D<i32, UnknownUnit>)> {
        if let Some(display) = self.displays.iter_mut().find(|i| i.get_bounds().contains(pos)) {
            let bounds = display.get_bounds();
            let pos = Point2D::new(pos.x - bounds.min.x, pos.y - bounds.min.y);

            Some((display, bounds, pos))
        } else {
            None
        }
    }

    pub fn get_layout(&self) -> Vec<IntRect> {
        self.displays.iter().map(|i| IntRect::from_origin_and_size(i.pos, Size2D::new(i.surface.width(), i.surface.height()))).collect()
    }

    pub fn mk_frame(&mut self, options: FrameOptions) -> syscall::Result<&Frame> {
        let id = self.frames.keys().max().unwrap_or(&0) + 1;
        let frame = Frame::new(options, id);

        match frame {
            Ok(frame) => self.frames.insert(id, frame),
            Err(err) => return Err(syscall::Error { errno: err }),
        };

        let Some(frame) = self.frames.get(&id) else {
            return Err(syscall::Error { errno: syscall::EINVAL });
        };

        self.events.lock().unwrap().push_back(PluginEvent::OnFrameCreate(frame.get_messenger()));

        Ok(frame)
    }

    fn sync_frame(&mut self, frame: usize) {
        // dbg!("Syncing Frame {}", frame);

        if let Some(frame) = self.frames.get(&frame) {
            for i in &mut self.displays {
                if i.get_bounds().contains(frame.pos) {
                    let pos = frame.pos - i.pos.to_vector();

                    i.sync(Some(SyncRect {
                        x: pos.x,
                        y: pos.y,
                        w: frame.surface.width(),
                        h: frame.surface.height(),
                    }));
                }
            }
        }

        self.displays.iter_mut().for_each(|i| i.draw_cursor(&self.cursor));
    }

    fn update_frame(&mut self, id: usize) -> syscall::Result<()> {
        if let Some(frame) = self.frames.get_mut(&id) {
            frame.last_update = Instant::now();
            frame.draw(&mut self.surface);

            self.events.lock().unwrap().push_back(PluginEvent::OnFrameUpdate(frame.get_messenger()));
        } else {
            return Err(syscall::Error::new(syscall::ENOENT));
        }

        if self.frames.contains_key(&id) {
            self.displays.iter_mut().for_each(|i| i.draw(&mut self.surface));
            self.sync_frame(id.clone());
        }

        Ok(())
    }

    pub fn close_frame(&mut self, id: usize) -> syscall::Result<()> {
        if !self.frames.contains_key(&id) {
            return Err(syscall::Error {
                errno: syscall::ENOENT,
            });
        }
        if let Some(frame) = self.frames.remove(&id) {
            self.events.lock().unwrap().push_back(PluginEvent::OnFrameDestroy(frame.get_messenger()));
        }

        Ok(())
    }

    pub fn get_frame_by_id(&self, id: usize) -> Option<&Frame> {
        self.frames.get(&id)
    }

    pub fn paint_buffer(&mut self, buffer: Vec<u32>, rect: Box2D<i32, UnknownUnit>) {
        todo!()
    }
}

impl<'a, 'b> SchemeMut for Compositor<'a, 'b> {
    fn open(&mut self, path: &str, flags: usize, uid: u32, gid: u32) -> syscall::Result<usize> {
        let options = match FrameOptions::from_string(path) {
            Ok(options) => options,
            Err(err) => return Err(syscall::Error {
                errno: syscall::EINVAL,
            }),
        };

        match self.mk_frame(options) {
            Ok(frame) => Ok(frame.id),
            Err(err) => Err(err)
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
            let map_pages = (map.offset + map.size + (syscall::PAGE_SIZE - 1)) / syscall::PAGE_SIZE;
            let (data_addr, len) = frame.mut_ptr();

            if map_pages * syscall::PAGE_SIZE >= len * std::mem::size_of::<u32>() {
                Ok((data_addr as usize) + map.offset)
            } else {
                Err(syscall::Error::new(syscall::EINVAL))
            }
        } else {
            Err(syscall::Error::new(syscall::ENOENT))
        }
    }

    fn fsync(&mut self, id: usize) -> syscall::Result<usize> {
        self.update_frame(id).map(|i| 0)
    }

    fn close(&mut self, id: usize) -> syscall::Result<usize> {
        self.close_frame(id).map(|i| 0)
    }
}
