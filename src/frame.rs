use std::slice;
use std::time::{Duration, Instant};

use euclid::{Box2D, Point2D, Size2D, UnknownUnit};
use raqote::{DrawOptions, DrawTarget, IntPoint, IntRect, SolidSource, Source};

use crate::bin::aligned_vec;
use crate::plugin::LuaFrame;

pub struct Frame<'a> {
    pub id: usize,
    pub pos: IntPoint,
    pub surface: DrawTarget<&'a mut [u32]>,
    pub last_update: Instant,
    // pub surface: Vec<u32>,
    // pub size: Size2D<i32, UnknownUnit>,
    pub parent: Option<usize>,
    pub title: String,
}

impl<'a> Frame<'a> {
    pub fn new(options: FrameOptions, id: usize) -> Result<Frame<'a>, i32> {
        let mut surface = unsafe {
            let ptr = aligned_vec(options.size.width as u32, options.size.height as u32).as_mut_ptr() as usize;

            let buffer = slice::from_raw_parts_mut(ptr as *mut u32, (options.size.width * options.size.height) as usize);
            DrawTarget::from_backing(options.size.width as i32, options.size.height as i32, buffer)
        };

        surface.clear(SolidSource::from_unpremultiplied_argb(0xff, 0xaa, 0xaa, 0xaa));

        Ok(Self {
            pos: options.pos.clone(),
            id,
            surface,
            // size: options.size.clone(),
            title: options.title,
            parent: options.parent,
            last_update: Instant::now(),
        })
    }

    pub(crate) fn mut_ptr(&mut self) -> (*mut u32, usize) {
        let ptr = self.surface.get_data_mut().as_mut_ptr();
        let len = self.surface.get_data().len();
        (ptr, len)
    }

    pub fn draw(&mut self, surface: &mut DrawTarget) {
        let size = Size2D::new(self.surface.width(), self.surface.height());
        let rect = Box2D::from_origin_and_size(self.pos, size);
        surface.copy_surface(&mut self.surface, rect.clone(), Point2D::new(0, 0));

        let elapsed = self.last_update.elapsed().as_secs_f64();
        if elapsed > 10. {
            let alpha = 255. * ((elapsed - 10.) / 5.).clamp(0., 0.5); // fade to 50% alpha over 2.5s

            surface.fill_rect(rect.min.x as f32,
                              rect.min.y as f32,
                              rect.width() as f32,
                              rect.height() as f32,
                              &Source::Solid(SolidSource::from_unpremultiplied_argb(alpha as u8, 0xff, 0xff, 0xff)),
                              &DrawOptions::default(),
            );
        }
    }

    pub fn lua_frame(&self) -> LuaFrame {
        LuaFrame {
            id: self.id,
            title: self.title.clone(),
            x: self.pos.x,
            y: self.pos.y,
            w: self.surface.width(),
            h: self.surface.height(),

            parent: None, // TODO: Implement
        }
    }
}

#[derive(Debug, Clone)]
pub enum ZIndex {
    Back,
    Auto,
    Front,
}

/// A list of options which can be used during the creation of a new frame.
#[derive(Debug, Clone)]
pub struct FrameOptions {
    pub min_size: Size2D<i32, UnknownUnit>,
    pub max_size: Size2D<i32, UnknownUnit>,
    pub size: Size2D<i32, UnknownUnit>,
    pub pos: Point2D<i32, UnknownUnit>,
    pub title: String,

    pub transparent: bool,

    pub can_minimise: bool,
    pub can_resize: bool,
    pub can_close: bool,
    pub z_lock: ZIndex,
    pub parent: Option<usize>,
}

fn parse_coord(str: &str) -> (i32, i32) {
    let mut parts = str.split(',');
    let x = parts.next().unwrap_or("").parse::<i32>().unwrap_or(0);
    let y = parts.next().unwrap_or("").parse::<i32>().unwrap_or(0);
    return (x, y);
}

impl FrameOptions {
    pub fn new(src: &str) -> Result<Self, String> {
        let mut options = FrameOptions {
            min_size: Size2D::new(0, 0),
            max_size: Size2D::new(i32::MAX, i32::MAX),
            size: Size2D::new(0, 0),
            pos: Point2D::new(0, 0),
            title: String::new(),
            transparent: false,
            can_minimise: false,
            can_resize: false,
            can_close: false,
            z_lock: ZIndex::Auto,
            parent: None,
        };

        for option in src.split('&') {
            match option {
                "minimise" => options.can_minimise = true,
                "resize" => options.can_resize = true,
                "close" => options.can_close = true,
                "transparent" => options.transparent = true,
                "z-lock=back" => options.z_lock = ZIndex::Back,
                "z-lock=front" => options.z_lock = ZIndex::Front,
                "parent" => {
                    let mut parts = option.split('=');
                    let _ = parts.next();
                    options.parent = match parts.next() {
                        Some(str) => Some(str.parse().unwrap_or(0usize)),
                        None => None
                    };
                }
                _ => {
                    let mut parts = option.split('=');
                    let key = parts.next().unwrap_or("");
                    let value = parts.next().unwrap_or("");
                    match key {
                        "min-size" => options.min_size = Size2D::from(parse_coord(value)).min(options.max_size),
                        "max-size" => options.max_size = Size2D::from(parse_coord(value)).max(options.min_size),
                        "size" => options.size = Size2D::from(parse_coord(value)).max(options.min_size).min(options.max_size),
                        "pos" => options.pos = Point2D::from(parse_coord(value)),
                        "title" => options.title = value.to_owned(),
                        key => {
                            return Err(format!("Invalid option {} or invalid value", key));
                        }
                    }
                }
            };
        }

        Ok(FrameOptions {
            min_size: options.min_size.min(options.max_size),
            max_size: options.max_size.max(options.min_size),
            size: options.size.clamp(options.min_size, options.max_size),
            ..options
        })
    }
}
