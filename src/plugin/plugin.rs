use std::fs::File;
use std::io::Read;

use crate::frame::{Frame, FrameMessenger, FrameOptions};

pub struct Plugin {
    pub source: File,
    pub lua: rlua::Lua,
}

macro_rules! handler {
    ($name:ident$(,$arg:ident: $val:ty)*) => {
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
}

macro_rules! set_handler {
    ($ctx:expr,$name:ident) => {
        if let Ok(handler) = $ctx.globals().get::<_, rlua::Function>(stringify!($name)) { $ctx.set_named_registry_value(stringify!($name), handler).unwrap(); };
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
            let mut source = String::new();
            self.source.read_to_string(&mut source).expect("Failed to read plugin source");
            if let Err(err) = ctx.load(&source).exec() {
                return Err(err);
            }

            // Frames
            set_handler!(ctx, on_frame_create);
            set_handler!(ctx, on_frame_destroy);
            set_handler!(ctx, on_frame_update);
            // Mouse
            set_handler!(ctx, on_mouse_move);
            set_handler!(ctx, on_mouse_down);
            set_handler!(ctx, on_mouse_up);
            set_handler!(ctx, on_mouse_scroll);
            // Keyboard
            set_handler!(ctx, on_key_down);
            set_handler!(ctx, on_key_up);
            // Plugin
            set_handler!(ctx, on_plugin_load);
            set_handler!(ctx, on_before_plugin_unload);

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
