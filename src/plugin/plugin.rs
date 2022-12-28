use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Receiver, Sender};

use crate::frame::{Frame, FrameMessenger, FrameOptions, FrameRequest};
use crate::plugin::{PluginRequest, PluginResponse};

/// Channels allow us to communicate across threads.
/// In the circumstance where Lua happens to call a provided method from another thread, this will cause race conditions, so we fall back to a callback-architecture
/// Implemented by messaging the main thread which does the execution and awaiting the response.
struct Channel {
    sender: Sender<PluginRequest>,
    receiver: Receiver<PluginResponse>,
    events: Arc<Mutex<HashMap<usize, rlua::RegistryKey>>>,
}

pub struct Plugin {
    pub source: File,
    pub lua: rlua::Lua,

    channel: Channel,
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
    pub fn new(path: &str, sender: Sender<PluginRequest>, receiver: Receiver<PluginResponse>) -> Result<Self, String> {
        Ok(Self {
            source: File::open(path).map_err(|_| format!("Unable to open plugin {}", path))?,
            lua: rlua::Lua::new(),
            channel: Channel {
                sender,
                receiver,
                events: Arc::new(Mutex::new(HashMap::new()))
            },
        })
    }

    pub fn run(&mut self) -> Result<(), String> {
        self.lua.context(|ctx| -> rlua::Result<()> {
            let mut source = String::new();

            {
                let globals = ctx.globals();

                let events = Arc::clone(&self.channel.events);
                let sender = self.channel.sender.clone();

                globals.set("create_frame", ctx.create_function(move |_, (options, on_create): (FrameOptions, rlua::Function)| -> rlua::Result<()> {
                    // let mut events = events.lock().unwrap();
                    // events.insert(events.len() as u128, ctx.create_registry_value(on_create)?);
                    sender.send(PluginRequest::CreateFrame(options)).unwrap();

                    Ok(())
                }).unwrap()).unwrap();
            };

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

    pub fn receive_responses(&mut self) {
        while let Ok(response) = self.channel.receiver.try_recv() {
            match response {
                PluginResponse::Frame(req, _) => {
                    if let Some(event) = self.channel.events.lock().unwrap().remove(&req) {
                        self.lua.context(|ctx| -> rlua::Result<()> {
                            ctx.registry_value::<rlua::Function>(&event).unwrap().call::<_, ()>(()).unwrap();
                            Ok(())
                        }).unwrap();
                    }
                }
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
