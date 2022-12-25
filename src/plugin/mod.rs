use std::borrow::Borrow;
use std::fs::File;
use std::io::Read;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use euclid::{Point2D, Size2D};
use rlua::{Context, Lua, Value};

use crate::compositor::Compositor;
use crate::frame::{Frame, FrameOptions, ZIndex};
pub use crate::plugin::data::LuaFrame;

mod data;

pub struct Plugin {
    pub script: File,
    ctx: Lua,
}

pub struct PluginManager {
    plugins: Vec<Plugin>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: vec![]
        }
    }

    pub fn load(&mut self, path: &str, ctx: Arc<Mutex<&mut Compositor>>) -> Result<&Plugin, String> {
        return if let Ok(plugin) = Plugin::new(path, ctx) {
            self.plugins.push(plugin);

            match self.plugins.last() {
                Some(plugin) => Ok(plugin),
                None => Err("Failed to load plugin".to_owned())
            }
        } else {
            Err("Failed to load plugin".to_owned())
        };
    }
}



// public plugin API
// these functions can be exported as _hooks_
// 1. on_frame_create(frame), on_frame_destroy(frame), on_frame_update(frame)
// 2. on_mouse_move(mouse), on_mouse_down(button), on_mouse_up(button), on_mouse_scroll(delta)
// 3. on_key_down(key), on_key_up(key)
// 5. on_plugin_load(plugin), on_before_plugin_unload(plugin)
// exported functions can be called from lua
// 1. create_frame(options), get_frame_by_id(id), close_frame(id)
// 2. get_mouse(), get_key(key)
// 3. paint_buffer(buffer, pos, size)
// objects
// 1. frame: {id, title, x, y, w, h, parent, get_buffer(), send_event(event), close()}
// 2. event: {type, x, y, button, key, delta}
// 3. buffer: u32[]
/// A plugin which is responsible for loading, unloading, managing and executing scripts and the public-facing API.
impl Plugin {
    pub fn new<'a>(src: &str, comp: Arc<Mutex<&mut Compositor>>) -> Result<Self, String> {
        if let Ok(mut script) = File::open(src) {
            let mut src = String::new();

            if let Err(_) = script.read_to_string(&mut src) {
                return Err("Failed to read plugin".to_owned());
            }

            let ctx = Lua::new();

            ctx.context(|ctx| {
                let globals = ctx.globals();

                globals.set("create_frame", ctx.create_function(|_, (options): (FrameOptions)| -> rlua::Result<LuaFrame> {
                    Err(rlua::Error::external("Creating frames is not yet implemented"))
                }).unwrap()).unwrap();

                // globals.set("get_frame_by_id", ctx.create_function(|_, ()| todo!()).unwrap()).unwrap();
                // globals.set("close_frame", ctx.create_function(|_, ()| todo!()).unwrap()).unwrap();
                // globals.set("get_mouse", ctx.create_function(|_, ()| todo!()).unwrap()).unwrap();
                // globals.set("get_key", ctx.create_function(|_, ()| todo!()).unwrap()).unwrap();
                // globals.set("paint_buffer", ctx.create_function(|_, ()| todo!()).unwrap()).unwrap();


                ctx.load(&src).exec().unwrap();
            });

            return Ok(Self {
                script,
                ctx,
            });
        }

        return Err(format!("Failed to load plugin {src}"));
    }
}
