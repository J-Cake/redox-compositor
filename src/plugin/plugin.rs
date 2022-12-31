use std::borrow::Borrow;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::ops::{Add, AddAssign};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use crate::frame::{Frame, FrameMessenger, FrameOptions, FrameRequest};
use crate::plugin::{PluginRequest, PluginResponse};

pub(crate) type MessageID = rlua::RegistryKey;

/// Channels allow us to communicate across threads.
/// In the circumstance where Lua happens to call a provided method from another thread, this will cause race conditions, so we fall back to a callback-architecture
/// Implemented by messaging the main thread which does the execution and awaiting the response.
struct Channel {
    sender: Sender<(MessageID, PluginRequest)>,
    receiver: Receiver<(MessageID, PluginResponse)>,
    event_id: Counter<usize>,
    reg_key: HashMap<MessageID, rlua::RegistryKey>,
}

struct Counter<T> where T: Add<Output=T> + Clone {
    index: T,
    step: T,
}

impl<T> Counter<T> where T: Add<Output=T> + Clone {
    pub fn new(start: T, step: T) -> Self {
        Self {
            index: start,
            step,
        }
    }

    fn next(&mut self) -> T {
        let i = self.index.clone();

        self.index = i.clone() + self.step.clone();

        return i;
    }
}

pub struct Plugin {
    pub source: File,
    pub lua: rlua::Lua,

    registry_key: Arc<rlua::RegistryKey>,
    channel: Channel,
}

macro_rules! handler {
    ($name:ident$(,$arg:ident: $val:ty)*) => {
        pub fn $name(&self$(, $arg:$val)*) {
            self.lua.context(|ctx| -> rlua::Result<()> {
                if let Ok(handler) = ctx.named_registry_value::<_, rlua::Function>(stringify!($name)) {
                    handler.call::<_, ()>(($($arg,)*))
                        .unwrap();
                }

                Ok(())
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
    pub fn new(path: &str, sender: Sender<(MessageID, PluginRequest)>, receiver: Receiver<(MessageID, PluginResponse)>) -> Result<Self, String> {
        let lua = rlua::Lua::new();
        let reg = lua.context(|ctx| ctx.create_registry_value(rlua::Value::Table(ctx.create_table().unwrap()))).unwrap();

        Ok(Self {
            source: File::open(path).map_err(|_| format!("Unable to open plugin {}", path))?,
            lua,
            registry_key: Arc::new(reg),
            channel: Channel {
                event_id: Counter::new(0usize, 1usize),
                sender,
                receiver,
                reg_key: HashMap::new(),
            },
        })
    }

    pub fn run(&mut self) -> Result<(), String> {
        let mut source = String::new();
        self.source.read_to_string(&mut source)
            .expect("Failed to read plugin source");

        let sender = self.channel.sender.clone();

        self.lua.context(|ctx| -> rlua::Result<()> {
            let globals = ctx.globals();

            globals.set("create_frame", ctx.create_function(move |ctx, (options, on_create): (FrameOptions, rlua::Function)| -> rlua::Result<()> {
                // Move the `on_create` callback into the plugin registry
                let registry_key = ctx.create_registry_value(on_create).unwrap();

                // The event handler is the plugin manager. It is responsible for performing the actions indicated by the `PluginRequest` object.
                // The RegistryKey is included in each request/response pair so the plugin is able to call the correct callback.
                // These are unique to the plugin's lua context and subsequently the plugin itself.

                // Dispatch the actual event
                sender.send((registry_key, PluginRequest::CreateFrame(options))).unwrap();

                Ok(())
            }).unwrap()).unwrap();

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

    pub fn receive_responses(&mut self) {
        while let Ok((id, response)) = self.channel.receiver.try_recv() {
            match response {
                PluginResponse::Frame(req) => self.lua.context(|ctx| if let Ok(handler) = ctx.registry_value::<rlua::Function>(&id) {
                    handler.call::<_, ()>((req, )).unwrap();
                }),
                _ => todo!()
            }
        }
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
