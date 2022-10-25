use std::cmp::Ordering;
use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{FromRawFd, RawFd};
use std::rc::Rc;
use std::thread;
use std::time::{Duration, Instant};

use lazy_static::lazy_static;
use raqote::{DrawTarget, IntPoint, IntRect, SolidSource};
use syscall::{O_CLOEXEC, O_CREAT, O_NONBLOCK, O_RDWR, Packet, SchemeMut};

use client::frame::*;

use crate::display::Display;
use crate::frame::Frame;

pub enum DisplayMode {
    Extend,
    Duplicate,
}

pub(crate) struct Desktop<'a> {
    pub(crate) displays: Vec<Display>,
    pub(crate) ctx: DrawTarget,
    pub(crate) area: Rect,
    pub(crate) display_mode: DisplayMode,
    // This is where frames are rendered into
    pub(crate) handler: File,

    pub(crate) frames: HashMap<usize, Frame>,
    pub(crate) frame_order: VecDeque<usize>,
    pub(crate) frame_id_counter: usize,
    pub(crate) unused_frame_ids: VecDeque<usize>,
    pub(crate) display_config: Option<Rc<DisplayConfiguration<'a>>>,
}

const SCHEME_NAME: &'static str = ":erika";
const MAX_FPS: Duration = Duration::from_nanos(1_000_000_000 / 60);

pub(crate) struct DisplayConfiguration<'a> {
    sizes: Vec<&'a Rect>,
    area: Rect,
}

impl<'a> DisplayConfiguration<'a> {
    pub(crate) fn fill_screen(&self, area: &Rect) -> Rect {
        fn dist(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
            ((x2 - x1) + (y2 - y1)).powf(0.5)
        }

        return self.sizes.iter()
            .max_by(|display1, display2| {
                let dist1 = dist(display1.x as f64, display1.y as f64, area.x as f64, area.y as f64);
                let dist2 = dist(display2.x as f64, display2.y as f64, area.x as f64, area.y as f64);

                return if dist1 > dist2 { Ordering::Greater } else if dist1 == dist2 { Ordering::Equal } else { Ordering::Less };
            })
            .map(|i| Rect {
                x: i.x,
                y: i.y,
                w: i.w,
                h: i.h,
            })
            .unwrap_or(self.area.clone());
    }
}

impl<'a> Desktop<'a> {
    pub(crate) fn new(displays: Vec<&str>) -> Self {
        let config = crate::config::load_config().unwrap();

        let handler = syscall::open(SCHEME_NAME, O_CREAT | O_RDWR | O_CLOEXEC | O_NONBLOCK)
            .map(|socket| unsafe { File::from_raw_fd(socket as RawFd) })
            .unwrap_or_else(|_| syscall::open(SCHEME_NAME, O_RDWR | O_CLOEXEC | O_NONBLOCK)
                .map(|socket| unsafe { File::from_raw_fd(socket as RawFd) })
                .unwrap());

        let mut area = Rect {
            x: i32::MAX,
            y: i32::MAX,
            w: u32::MIN,
            h: u32::MIN,
        };
        let mut display_map = displays.iter().map(|i| {
            let display = Self::load_display(i)
                .unwrap();

            area.x = area.x.min(display.area.x);
            area.y = area.y.min(display.area.y);
            area.w = area.w.max(display.area.w);
            area.h = area.h.max(display.area.h);

            return display;
        }).collect();

        let mut ctx = Self {
            displays: display_map,
            ctx: DrawTarget::new(area.w as i32, area.h as i32),
            area,
            handler,
            display_mode: DisplayMode::Extend,
            frames: HashMap::new(),
            frame_id_counter: 0,
            unused_frame_ids: VecDeque::from([]),
            frame_order: VecDeque::new(),
            display_config: None,
        };

        return ctx;
    }

    pub(crate) fn get_display_config(&mut self) -> Rc<DisplayConfiguration<'a>> {
        if let Some(config) = &self.display_config {
            return config.clone();
        }

        let config = Rc::new(DisplayConfiguration {
            sizes: (self.displays).iter().map(|i| &i.area).collect(),
            area: self.area.clone(),
        });
        let _config = config.clone();
        self.display_config = Some(config);
        return _config;
    }

    pub(crate) fn run(&mut self) {
        self.ctx.clear(SolidSource::from_unpremultiplied_argb(0xff, 0, 0, 0));
        self.flush();

        loop {
            let start = Instant::now();

            let mut packet = Packet::default();

            match self.handler.read(&mut packet) {
                Ok(len) => if len <= 0 { break; } else {
                    self.handle(&mut packet);
                    self.handler.write(&packet)
                        .unwrap();

                    self.flush();
                },
                _ => {}
            };

            // limit poll rates to ~60x a second
            let elapsed = start.elapsed();
            if elapsed < MAX_FPS {
                thread::sleep(MAX_FPS - elapsed);
            }
        }
    }

    pub(crate) fn flush(&mut self) {
        match self.handler.sync_all() {
            Ok(..) => {}
            Err(e) => eprintln!("{:?}", e)
        }

        match self.display_mode {
            DisplayMode::Extend => {
                for display in &mut self.displays {
                    let min = IntPoint::new(display.area.x, display.area.y);
                    let max = IntPoint::new(display.area.w as i32, display.area.h as i32);

                    display.ctx.copy_surface(&self.ctx, IntRect::new(IntPoint::zero(), max), min);
                    display.buffer.sync_all()
                        .unwrap();
                }
            }
            DisplayMode::Duplicate => {}
        }
    }

    pub(crate) fn next_id(&mut self) -> usize {
        match self.unused_frame_ids.pop_front() {
            Some(id) => id,
            None => {
                self.frame_id_counter += 1;
                self.frame_id_counter
            }
        }
    }

    pub(crate) fn push_frame(&mut self, frame: Frame) -> &mut Frame {
        let id = frame.id.clone();
        self.frames.insert(frame.id.clone(), frame);
        self.frame_order.push_back(id.clone());
        return self.frames.get_mut(&id).unwrap();
    }
}