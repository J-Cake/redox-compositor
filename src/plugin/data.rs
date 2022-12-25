use std::borrow::Borrow;

use euclid::{Point2D, Size2D};
use rlua::{Context, FromLua, ToLua, Value};
use rlua::prelude::LuaUserData;

use crate::frame::{Frame, FrameOptions, ZIndex};

impl<'lua> rlua::FromLua<'lua> for FrameOptions {
    fn from_lua(lua_value: Value<'lua>, lua: Context<'lua>) -> rlua::Result<Self> {
        match lua_value {
            Value::Table(table) => Ok(Self {
                // TODO: Finish
                min_size: Size2D::new(0, 0),
                max_size: Size2D::new(0, 0),
                size: Size2D::new(0, 0),
                pos: Point2D::new(0, 0),
                title: table.get("title").unwrap_or("".to_owned()),
                transparent: table.get("transparent").unwrap_or(false),
                can_minimise: table.get("can_minimise").unwrap_or(false),
                can_resize: table.get("can_resize").unwrap_or(false),
                can_close: table.get("can_close").unwrap_or(false),
                z_lock: match table.get::<String, String>("z_lock".into()).unwrap_or("".to_owned()).borrow() {
                    "back" => ZIndex::Back,
                    "front" => ZIndex::Front,
                    _ => ZIndex::Auto,
                },
                parent: table.get("parent").unwrap_or(None),
            }),
            _ => Err(rlua::Error::FromLuaConversionError {
                from: "Value",
                to: "FrameOptions",
                message: Some("Value is not a table".to_owned()),
            })
        }
    }
}

pub struct LuaFrame<'a> {
    pub id: usize,
    pub title: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub parent: Option<&'a LuaFrame<'a>>,
}

impl<'a, 'lua> ToLua<'lua> for LuaFrame<'a> {
    fn to_lua(self, lua: Context<'lua>) -> rlua::Result<Value<'lua>> {
        Ok(Value::Table(lua.create_table_from(vec![("key".to_owned(), "value".to_owned())]).unwrap()))
    }
}

impl<'a, 'lua> FromLua<'lua> for LuaFrame<'a> {
    fn from_lua(lua_value: Value<'lua>, lua: Context<'lua>) -> rlua::Result<Self> {
        match lua_value {
            Value::Table(value) => Ok(LuaFrame {
                // TODO: Finish
                id: value.get("id").unwrap_or(0usize),
                title: value.get("title").unwrap_or("".to_owned()),
                x: 0,
                y: 0,
                w: 0,
                h: 0,
                parent: None,
            }),
            _ => Err(rlua::Error::FromLuaConversionError {
                from: "Value",
                to: "LuaFrame",
                message: Some("Value is not a table".to_owned()),
            })
        }
    }
}
