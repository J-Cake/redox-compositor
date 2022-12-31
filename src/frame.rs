use std::{mem, slice};
use std::time::{Duration, Instant};

use euclid::{Box2D, Point2D, Size2D, UnknownUnit};
use raqote::{DrawOptions, DrawTarget, IntPoint, IntRect, SolidSource, Source};
use rlua::{Context, FromLua, Table, ToLua, Value};
use rlua::prelude::LuaTable;
use syscall::PAGE_SIZE;

use crate::bin::aligned_vec;

pub struct Frame<'a> {
    pub id: usize,
    pub pos: IntPoint,
    pub surface: DrawTarget<&'a mut [u32]>,
    pub last_update: Instant,
    pub parent: Option<usize>,
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct FrameMessenger {
    pub id: usize,
    pub pos: IntPoint,
    pub size: Size2D<i32, UnknownUnit>,
    pub last_update: Instant,
    pub parent: Option<usize>,
    pub title: String,
}

/// Frames are _Windows_ to the client. They are anything the compositor displays. They capture and use input, and can be placed arbitrarily on the screen.
/// Applications controlling frames typically use the following architecture:
///  * Each client opens two buffers - front and back buffers.
///  * The client draws to the back buffer, and then swaps the buffers.
/// Optionally, the client can request a third, unrelated buffer be populated with the contents beneath the frame.
/// - This can be used to create systems ranging from blurred backgrounds to screen readers.
impl<'a> Frame<'a> {
    pub fn new(options: FrameOptions, id: usize) -> Result<Frame<'a>, i32> {
        let Size2D { width, height, .. } = options.size;
        let mut surface = DrawTarget::from_backing(width, height, unsafe {
            let layout = std::alloc::Layout::from_size_align(mem::size_of::<u32>() * (width * height) as usize, PAGE_SIZE).unwrap();
            let ptr = std::alloc::alloc(layout) as *mut u32;
            std::slice::from_raw_parts_mut::<u32>(ptr, (width * height) as usize)
        });

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

    pub fn get_messenger(&self) -> FrameMessenger {
        FrameMessenger {
            id: self.id,
            pos: self.pos,
            size: Size2D::new(self.surface.width(), self.surface.height()),
            last_update: self.last_update,
            parent: self.parent,
            title: self.title.clone(),
        }
    }
}

impl<'lua> ToLua<'lua> for FrameMessenger {
    fn to_lua(self, lua: Context<'lua>) -> rlua::Result<Value<'lua>> {
        let val = lua.create_table().unwrap();

        let pos = lua.create_table().unwrap();
        pos.set("x", self.pos.x).unwrap();
        pos.set("y", self.pos.y).unwrap();
        val.set("pos", pos).unwrap();

        let size = lua.create_table().unwrap();
        size.set("width", self.size.width).unwrap();
        size.set("height", self.size.height).unwrap();
        val.set("size", size).unwrap();

        val.set("id", self.id).unwrap();
        val.set("title", self.title).unwrap();
        val.set("parent", self.parent).unwrap();
        val.set("last_update", self.last_update.elapsed().as_secs_f64()).unwrap();

        Ok(Value::Table(val))
    }
}

impl<'lua> FromLua<'lua> for FrameMessenger {
    fn from_lua(lua_value: Value<'lua>, lua: Context<'lua>) -> rlua::Result<Self> {
        if let Value::Table(table) = lua_value {
            let pos = table.get::<_, LuaTable>("pos").unwrap();
            let x = pos.get::<_, i32>("x").unwrap();
            let y = pos.get::<_, i32>("y").unwrap();

            let size = table.get::<_, LuaTable>("size").unwrap();
            let width = size.get::<_, i32>("width").unwrap();
            let height = size.get::<_, i32>("height").unwrap();

            let id = table.get::<_, usize>("id").unwrap();
            let title = table.get::<_, String>("title").unwrap();
            let parent = table.get::<_, Option<usize>>("parent").unwrap();
            let last_update = table.get::<_, f64>("last_update").unwrap();

            Ok(Self {
                id,
                pos: IntPoint::new(x, y),
                size: Size2D::new(width, height),
                title,
                parent,
                last_update: Instant::now() - Duration::from_secs_f64(last_update),
            })
        } else {
            Err(rlua::Error::FromLuaConversionError {
                from: "Value",
                to: "FrameMessenger",
                message: Some("Value is not a table".to_string()),
            })
        }
    }
}

#[derive(Debug, Clone)]
pub enum ZIndex {
    Back,
    Auto,
    Front,
}

impl<'lua> ToLua<'lua> for ZIndex {
    fn to_lua(self, lua: Context<'lua>) -> rlua::Result<Value<'lua>> {
        match self {
            ZIndex::Back => Ok(Value::String(lua.create_string("back").unwrap())),
            ZIndex::Auto => Ok(Value::String(lua.create_string("auto").unwrap())),
            ZIndex::Front => Ok(Value::String(lua.create_string("front").unwrap())),
        }
    }
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

impl FrameOptions {
    fn normalise(self) -> Self {
        Self {
            min_size: self.min_size,
            max_size: self.max_size,
            size: self.size.clamp(self.min_size, self.max_size),
            pos: Point2D::new(self.pos.x.max(0), self.pos.y.max(0)),
            title: self.title.clone(),
            transparent: self.transparent,
            can_minimise: self.can_minimise,
            can_resize: self.can_resize,
            can_close: self.can_close,
            z_lock: self.z_lock.clone(),
            parent: self.parent,
        }
    }
}

impl Default for FrameOptions {
    fn default() -> Self {
        Self {
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
        }
    }
}

impl<'lua> FromLua<'lua> for FrameOptions {
    fn from_lua(lua_value: Value<'lua>, lua: Context<'lua>) -> rlua::Result<Self> {
        match lua_value {
            Value::Table(value) => Ok(Self {
                min_size: value.get::<_, Table>("min_size").map(|v| Size2D::new(v.get::<_, i32>("width").unwrap_or_default(), v.get::<_, i32>("height").unwrap_or_default())).unwrap_or_default(),
                max_size: value.get::<_, Table>("max_size").map(|v| Size2D::new(v.get::<_, i32>("width").unwrap_or(i32::MAX), v.get::<_, i32>("height").unwrap_or(i32::MAX))).unwrap_or(Size2D::new(i32::MAX, i32::MAX)),
                size: value.get::<_, Table>("size").map(|v| Size2D::new(v.get::<_, i32>("width").unwrap_or_default(), v.get::<_, i32>("height").unwrap_or_default())).unwrap_or_default(),
                pos: value.get::<_, Table>("pos").map(|v| Point2D::new(v.get::<_, i32>("x").unwrap_or_default(), v.get::<_, i32>("y").unwrap_or_default())).unwrap_or_default(),
                title: value.get("title").unwrap_or_default(),
                transparent: value.get("transparent").unwrap_or_default(),
                can_minimise: value.get("can_minimise").unwrap_or_default(),
                can_resize: value.get("can_resize").unwrap_or_default(),
                can_close: value.get("can_close").unwrap_or_default(),
                z_lock: {
                    match value.get("z_lock") {
                        Ok(Value::String(s)) => match s.to_str().unwrap() {
                            "back" => ZIndex::Back,
                            "auto" => ZIndex::Auto,
                            "front" => ZIndex::Front,
                            _ => ZIndex::Auto,
                        },
                        _ => ZIndex::Auto,
                    }
                },
                parent: value.get("parent").unwrap_or_default(),
            }.normalise()),
            _ => Err(rlua::Error::FromLuaConversionError {
                message: Some("Expected Table".to_owned()),
                from: "FrameOptions",
                to: "FrameOptions",
            })
        }
    }
}

fn parse_coord(str: &str) -> (i32, i32) {
    let mut parts = str.split(',');
    let x = parts.next().unwrap_or("").parse::<i32>().unwrap_or(0);
    let y = parts.next().unwrap_or("").parse::<i32>().unwrap_or(0);
    return (x, y);
}

impl FrameOptions {
    pub fn from_string(src: &str) -> Result<Self, String> {
        let mut options = FrameOptions::default();

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

#[derive(Debug, Clone)]
pub enum FrameRequest {
    Create(FrameOptions),
    Destroy(usize),
    // TODO: Add request to fill background buffer
    SetTitle(usize, String),
    SetSize(usize, Size2D<i32, UnknownUnit>),
    SetPos(usize, Point2D<i32, UnknownUnit>),
    SetParent(usize, usize),
    SetZIndex(usize, ZIndex),
    SetCanMinimise(usize, bool),
    SetCanResize(usize, bool),
    SetCanClose(usize, bool),
    SetTransparent(usize, bool),
}

#[derive(Debug, Clone)]
pub enum FrameEvent<'a> {
    Created(&'a FrameMessenger),
    Destroyed(usize),
    TitleChanged(usize, String),
    SizeChanged(usize, Size2D<i32, UnknownUnit>),
    PosChanged(usize, Point2D<i32, UnknownUnit>),
    ParentChanged(usize, usize),
    ZIndexChanged(usize, ZIndex),
    CanMinimiseChanged(usize, bool),
    CanResizeChanged(usize, bool),
    CanCloseChanged(usize, bool),
    TransparentChanged(usize, bool),
}

impl<'lua> FromLua<'lua> for FrameRequest {
    fn from_lua(value: Value<'lua>, lua: Context<'lua>) -> rlua::Result<Self> {
        if let Value::Table(value) = value {
            let action = value.get::<_, String>("action").unwrap();
            let id = value.get::<_, usize>("id").unwrap();

            match action.as_str() {
                "create" => {
                    let options = value.get::<_, String>("options").unwrap();
                    let options = FrameOptions::from_string(&options).unwrap();
                    Ok(FrameRequest::Create(options))
                }
                "destroy" => Ok(FrameRequest::Destroy(id)),
                "set-title" => {
                    let title = value.get::<_, String>("title").unwrap();
                    Ok(FrameRequest::SetTitle(id, title))
                }
                "set-size" => {
                    let size = value.get::<_, Table>("size").unwrap();
                    let size = Size2D::new(size.get("x").unwrap(), size.get("y").unwrap());
                    Ok(FrameRequest::SetSize(id, size))
                }
                "set-pos" => {
                    let pos = value.get::<_, Table>("pos").unwrap();
                    let pos = Point2D::new(pos.get("x").unwrap(), pos.get("y").unwrap());
                    Ok(FrameRequest::SetPos(id, pos))
                }
                "set-parent" => {
                    let parent = value.get::<_, usize>("parent").unwrap();
                    Ok(FrameRequest::SetParent(id, parent))
                }
                "set-z-index" => {
                    let z_index = value.get::<_, String>("z-index").unwrap();
                    let z_index = match z_index.as_str() {
                        "back" => ZIndex::Back,
                        "front" => ZIndex::Front,
                        _ => ZIndex::Auto
                    };
                    Ok(FrameRequest::SetZIndex(id, z_index))
                }
                "set-can-minimise" => {
                    let can_minimise = value.get::<_, bool>("can-minimise").unwrap();
                    Ok(FrameRequest::SetCanMinimise(id, can_minimise))
                }
                "set-can-resize" => {
                    let can_resize = value.get::<_, bool>("can-resize").unwrap();
                    Ok(FrameRequest::SetCanResize(id, can_resize))
                }
                "set-can-close" => {
                    let can_close = value.get::<_, bool>("can-close").unwrap();
                    Ok(FrameRequest::SetCanClose(id, can_close))
                }
                "set-transparent" => {
                    let transparent = value.get::<_, bool>("transparent").unwrap();
                    Ok(FrameRequest::SetTransparent(id, transparent))
                }
                _ => Err(rlua::Error::FromLuaConversionError {
                    from: "FrameRequest",
                    to: "FrameRequest",
                    message: Some(format!("Unrecognised discriminant: '{}'", action)),
                })
            }
        } else {
            return Err(rlua::Error::FromLuaConversionError {
                from: "FrameRequest",
                to: "FrameRequest",
                message: Some(format!("Expected table")),
            });
        }
    }
}

impl<'lua, 'a> ToLua<'lua> for FrameEvent<'a> {
    fn to_lua(self, lua: Context<'lua>) -> rlua::Result<Value<'lua>> {
        let table = lua.create_table().unwrap();

        table.set("event", match self {
            FrameEvent::Created(_) => "created",
            FrameEvent::Destroyed(_) => "destroyed",
            FrameEvent::TitleChanged(_, _) => "title-changed",
            FrameEvent::SizeChanged(_, _) => "size-changed",
            FrameEvent::PosChanged(_, _) => "pos-changed",
            FrameEvent::ParentChanged(_, _) => "parent-changed",
            FrameEvent::ZIndexChanged(_, _) => "z-index-changed",
            FrameEvent::CanMinimiseChanged(_, _) => "can-minimise-changed",
            FrameEvent::CanResizeChanged(_, _) => "can-resize-changed",
            FrameEvent::CanCloseChanged(_, _) => "can-close-changed",
            FrameEvent::TransparentChanged(_, _) => "transparent-changed",
        }).unwrap();

        match self {
            FrameEvent::Created(frame) => table.set("frame", frame.clone()).unwrap(),
            FrameEvent::Destroyed(id) => table.set("id", id).unwrap(),
            FrameEvent::TitleChanged(id, title) => {
                table.set("id", id).unwrap();
                table.set("title", title).unwrap();
            }
            FrameEvent::SizeChanged(id, size) => {
                table.set("id", id).unwrap();
                table.set("size", {
                    let mut table = lua.create_table().unwrap();
                    table.set("width", size.width).unwrap();
                    table.set("height", size.height).unwrap();
                    table
                }).unwrap();
            }
            FrameEvent::PosChanged(id, pos) => {
                table.set("id", id).unwrap();
                table.set("pos", {
                    let mut table = lua.create_table().unwrap();
                    table.set("x", pos.x).unwrap();
                    table.set("y", pos.y).unwrap();
                    table
                }).unwrap();
            }
            FrameEvent::ParentChanged(id, parent) => {
                table.set("id", id).unwrap();
                table.set("parent", parent).unwrap();
            }
            FrameEvent::ZIndexChanged(id, z_index) => {
                table.set("id", id).unwrap();
                table.set("z-index", z_index).unwrap();
            }
            FrameEvent::CanMinimiseChanged(id, can_minimise) => {
                table.set("id", id).unwrap();
                table.set("can-minimise", can_minimise).unwrap();
            }
            FrameEvent::CanResizeChanged(id, can_resize) => {
                table.set("id", id).unwrap();
                table.set("can-resize", can_resize).unwrap();
            }
            FrameEvent::CanCloseChanged(id, can_close) => {
                table.set("id", id).unwrap();
                table.set("can-close", can_close).unwrap();
            }
            FrameEvent::TransparentChanged(id, transparent) => {
                table.set("id", id).unwrap();
                table.set("transparent", transparent).unwrap();
            }
        }

        Ok(Value::Table(table))
    }
}
