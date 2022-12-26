use std::fs::File;
use std::io::Read;

use crate::frame::{Frame, FrameMessenger, FrameOptions};

pub struct Plugin {
    pub source: File,
    pub lua: rlua::Lua,
}

macro_rules! handler {
    ($name:ident, $($arg:ident: $val:ty),*) => {
        pub fn $name(&self$(, $arg:$val)*) {
            self.lua.context(|ctx| -> rlua::Result<()> {
                match ctx.named_registry_value::<_, rlua::Function>(stringify!($name)) {
                    Err(_) => Ok(()),
                    Ok(handler) => {
                        handler.call::<_, ()>(($($arg,)*))
                            .unwrap();
                        Ok(())
                    },
                }
            }).unwrap()
        }
    };

    ($name:ident) => {
        pub fn $name(&self) {
            self.lua.context(|ctx| -> rlua::Result<()> {
                match ctx.named_registry_value::<_, rlua::Function>(stringify!($name)) {
                    Err(_) => Ok(()),
                    Ok(handler) => {
                        handler.call::<_, ()>(())
                            .unwrap();
                        Ok(())
                    },
                }
            }).unwrap()
        }
    }
}

impl Plugin {
    pub fn new(path: &str) -> Result<Self, String> {
        Ok(Self {
            source: File::open(path).map_err(|_| format!("Unable to open plugin {}", path))?,
            lua: rlua::Lua::new(),
        })
    }

    pub fn run(&mut self) -> Result<(), String> {
        self.lua.context(|ctx| -> rlua::Result<()> {
            let globals = ctx.globals();
            let mut source = String::new();
            self.source.read_to_string(&mut source).expect("Failed to read plugin source");
            let Ok(_) = ctx.load(&source).exec() else { return Err(rlua::Error::BindError); };

            // Frames
            if let Ok(on_frame_create) = globals.get::<_, rlua::Function>("on_frame_create") { ctx.set_named_registry_value("on_frame_create", on_frame_create).unwrap(); };
            if let Ok(on_frame_destroy) = globals.get::<_, rlua::Function>("on_frame_destroy") { ctx.set_named_registry_value("on_frame_destroy", on_frame_destroy).unwrap(); };
            if let Ok(on_frame_update) = globals.get::<_, rlua::Function>("on_frame_update") { ctx.set_named_registry_value("on_frame_update", on_frame_update).unwrap(); };

            // Mouse
            if let Ok(on_mouse_move) = globals.get::<_, rlua::Function>("on_mouse_move") { ctx.set_named_registry_value("on_mouse_move", on_mouse_move).unwrap(); };
            if let Ok(on_mouse_down) = globals.get::<_, rlua::Function>("on_mouse_down") { ctx.set_named_registry_value("on_mouse_down", on_mouse_down).unwrap(); };
            if let Ok(on_mouse_up) = globals.get::<_, rlua::Function>("on_mouse_up") { ctx.set_named_registry_value("on_mouse_up", on_mouse_up).unwrap(); };
            if let Ok(on_mouse_scroll) = globals.get::<_, rlua::Function>("on_mouse_scroll") { ctx.set_named_registry_value("on_mouse_scroll", on_mouse_scroll).unwrap(); };

            // Keyboard
            if let Ok(on_key_down) = globals.get::<_, rlua::Function>("on_key_down") { ctx.set_named_registry_value("on_key_down", on_key_down).unwrap(); };
            if let Ok(on_key_up) = globals.get::<_, rlua::Function>("on_key_up") { ctx.set_named_registry_value("on_key_up", on_key_up).unwrap(); };

            // Plugin
            if let Ok(on_plugin_load) = globals.get::<_, rlua::Function>("on_plugin_load") { ctx.set_named_registry_value("on_plugin_load", on_plugin_load).unwrap(); };
            if let Ok(on_before_plugin_unload) = globals.get::<_, rlua::Function>("on_before_plugin_unload") { ctx.set_named_registry_value("on_before_plugin_unload", on_before_plugin_unload).unwrap(); };

            Ok(())
        }).unwrap();

        // call the plugin load handler
        self.on_plugin_load();

        Ok(())
    }

    handler!(on_frame_create, frame: FrameMessenger);
    handler!(on_frame_destroy, frame: FrameMessenger);
    handler!(on_frame_update, frame: FrameMessenger);

    handler!(on_mouse_move, x: i32, y: i32);
    handler!(on_mouse_down, button: u8);
    handler!(on_mouse_up, button: u8);
    handler!(on_mouse_scroll, delta_x: f32, delta_y: f32);

    handler!(on_key_down, key: u8);
    handler!(on_key_up, key: u8);

    handler!(on_plugin_load);
    handler!(on_before_plugin_unload);
}
