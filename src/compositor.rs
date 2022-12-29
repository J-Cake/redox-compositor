use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::{Read, Write};
use std::mem;
use std::os::fd::{FromRawFd, RawFd};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use euclid::Size2D;
use lazy_static::lazy_static;
use raqote::{DrawTarget, IntPoint, IntRect, SolidSource};
use raqote::Source::Solid;
use syscall::{Map, O_NONBLOCK, Packet, SchemeMut};

use crate::config::Config;
use crate::display::Display;
use crate::frame::{Frame, FrameOptions};
use crate::plugin;
use crate::plugin::{PluginEvent, PluginManager};

pub struct Compositor<'a, 'b> {
    pub displays: Vec<Display<'a>>,

    pub frames: HashMap<usize, Frame<'b>>,

    pub surface: DrawTarget,

    pub cursor: IntPoint,

    pub scheme: File,

    pub plugin_manager: PluginManager,
}

pub const SCHEME_NAME: &'static str = ":comp";
pub const MAX_FPS: Duration = Duration::from_nanos(1_000_000_000 / 60);

impl<'a, 'b> Compositor<'a, 'b> {
    pub fn new(config: Config) -> Result<Self, String> {
        let displays: Vec<Display> = config.displays.iter()
            .map(|(name, pos)| Display::new(&name, &pos)
                .expect("Failed to create display"))
            .collect();

        println!("Created {} displays", displays.len());

        let mut min = (0, 0);
        let mut max = (0, 0);

        displays.iter().for_each(|i| {
            min = (i.pos.x.min(min.0), i.pos.y.min(min.1));
            max = ((i.pos.x + i.surface.width()).max(max.0), (i.pos.y + i.surface.height()).max(max.1));
        });

        Ok(Compositor {
            displays,
            frames: HashMap::new(),
            surface: DrawTarget::new(max.0 - min.0, max.1 - min.1),
            cursor: IntPoint::new(0, 0),
            scheme: syscall::open(SCHEME_NAME, syscall::O_CREAT | syscall::O_RDWR | syscall::O_CLOEXEC | O_NONBLOCK)
                .map(|socket| unsafe { File::from_raw_fd(socket as RawFd) })
                .unwrap_or_else(|_| {
                    syscall::open(SCHEME_NAME, syscall::O_RDWR | syscall::O_CLOEXEC | O_NONBLOCK)
                        .map(|socket| unsafe { File::from_raw_fd(socket as RawFd) })
                        .unwrap()
                }),
            plugin_manager: PluginManager::new(),
        })
    }

    pub fn run(&mut self) {
        loop {
            if self.frames.len() > 0 {
                // To indicate that the reference is still valid.
                // println!("SizeOf Surface: {:#?}\n\n", mem::size_of_val(self.surface));

                let now = Instant::now();
                let mut packet = Packet::default();

                if let Ok(len) = self.scheme.read(&mut packet) {
                    if len > 0 {
                        self.handle(&mut packet);
                        self.scheme.write(&packet).unwrap();
                    }
                }

                // self.plugin_manager.read_requests();

                self.draw();

                let elapsed = now.elapsed();
                if elapsed < MAX_FPS {
                    std::thread::sleep(MAX_FPS - elapsed);
                }
            } else {
                self.surface.clear(SolidSource::from_unpremultiplied_argb(0xff, 0, 0, 0));
                let mut packet = Packet::default();
                if let Ok(num) = self.scheme.read(&mut packet) {
                    self.handle(&mut packet);
                    self.scheme.write(&packet).unwrap();
                }

                // self.plugin_manager.read_requests();

                self.draw();
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }

    pub fn draw(&mut self) {
        self.surface.clear(SolidSource::from_unpremultiplied_argb(0xff, 0, 0, 0));

        self.frames.values_mut().for_each(|i| i.draw(&mut self.surface));
        self.displays.iter_mut().for_each(|i| i.draw(&mut self.surface));
    }

    pub fn get_layout(&self) -> Vec<IntRect> {
        self.displays.iter().map(|i| IntRect::from_origin_and_size(i.pos, Size2D::new(i.surface.width(), i.surface.height()))).collect()
    }

    pub fn load_plugins(&mut self, plugins: &Vec<String>) -> Result<(), String> {
        for i in plugins {
            let Ok(plugin) = self.plugin_manager.load(i) else {
                return Err(format!("Failed to load plugin {}", i));
            };
        }

        Ok(())
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

        self.plugin_manager.event(PluginEvent::OnFrameCreate(frame.get_messenger()));

        Ok(frame)
    }

    pub fn update(&mut self, id: usize) -> syscall::Result<()> {
        if let Some(frame) = self.frames.get_mut(&id) {
            frame.last_update = Instant::now();
            frame.draw(&mut self.surface);
            self.plugin_manager.event(PluginEvent::OnFrameUpdate(frame.get_messenger()));
        } else {
            return Err(syscall::Error::new(syscall::ENOENT));
        }

        Ok(())
    }

    pub fn close(&mut self, id: usize) -> syscall::Result<()> {
        if !self.frames.contains_key(&id) {
            return Err(syscall::Error {
                errno: syscall::ENOENT,
            });
        }
        if let Some(frame) = self.frames.remove(&id) {
            self.plugin_manager.event(PluginEvent::OnFrameDestroy(frame.get_messenger()));
        }

        Ok(())
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

            if map_pages * syscall::PAGE_SIZE >= len * mem::size_of::<u32>() {
                Ok((data_addr as usize) + map.offset)
            } else {
                Err(syscall::Error::new(syscall::EINVAL))
            }
        } else {
            Err(syscall::Error::new(syscall::ENOENT))
        }
    }

    fn fsync(&mut self, id: usize) -> syscall::Result<usize> {
        self.update(id).map(|i| 0)
    }

    fn close(&mut self, id: usize) -> syscall::Result<usize> {
        self.close(id).map(|i| 0)
    }
}
