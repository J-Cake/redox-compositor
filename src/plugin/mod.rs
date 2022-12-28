use std::sync::{mpsc, Mutex};
use std::sync::mpsc::{Receiver, Sender};

use euclid::{Point2D, Size2D, UnknownUnit};
use raqote::IntPoint;

use crate::compositor::Compositor;
use crate::frame::{FrameMessenger, FrameOptions};
use crate::plugin::plugin::Plugin;

mod plugin;

/// # public plugin API
/// ## these functions can be exported as _hooks_
/// 1. Frame
///     * `OnFrameCreate(frame)`
///     * `OnFrameDestroy(frame)`
///     * `OnFrameUpdate(frame)`
/// 2. Mouse
///     * `OnMouseMove(mouse)`
///     * `OnMouseDown(button)`
///     * `OnMouseUp(button)`
///     * `OnMouseScroll(delta)`
/// 3. Keyboard
///     * `OnKeyDown(key)`
///     * `OnKeyUp(key)`
/// 5. Plugin
///     * `OnPluginLoad(plugin)`
///     * `OnBeforePluginUnload(plugin)`
///
/// ## these functions can be called from the plugin
/// 1. Frames
///     * `create_frame(options)`
///     * `get_frame_by_id(id)`
///     * `close_frame(id)`
/// 2. Input
///     * `get_mouse() -> Mouse`
///     * `get_keys() -> Keys`
/// 3. Painting
///     * `paint_buffer(buffer, pos, size)`
///
/// ## objects
/// * `Frame {id, title, x, y, w, h, parent() -> Frame, get_buffer() -> Buffer, send_event(Event), close()}`
/// * `Event {type, x, y, button, key, delta}`
/// * `Buffer u32[]`
/// * `Mouse {x, y, buttons, scroll_delta}`
/// * `Keys {pressed, released}`

struct Channel {
    pub sender: Sender<PluginResponse>,
    pub receiver: Receiver<PluginRequest>,
}

pub struct PluginManager {
    loaded: Vec<(Plugin, Channel)>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            loaded: vec![]
        }
    }

    pub fn load(&mut self, path: &str) -> Result<(), String> {
        let request = mpsc::channel::<PluginRequest>();
        let response = mpsc::channel::<PluginResponse>();

        let mut plugin = Plugin::new(path, request.0, response.1)?;
        plugin.run().unwrap();
        self.loaded.push((plugin, Channel {
            sender: response.0,
            receiver: request.1,
        }));

        Ok(())
    }

    pub fn event(&self, event: PluginEvent) {
        for (plugin,_) in &self.loaded {
            match event.clone() {
                PluginEvent::OnFrameCreate(frame) => plugin.on_frame_create(frame),
                PluginEvent::OnFrameDestroy(frame) => plugin.on_frame_destroy(frame),
                PluginEvent::OnFrameUpdate(frame) => plugin.on_frame_update(frame),
                PluginEvent::OnMouseMove(x, y) => plugin.on_mouse_move(x, y),
                PluginEvent::OnMouseDown(btn) => plugin.on_mouse_down(btn),
                PluginEvent::OnMouseUp(btn) => plugin.on_mouse_up(btn),
                PluginEvent::OnMouseScroll(dx, dy) => plugin.on_mouse_scroll(dx, dy),
                PluginEvent::OnKeyDown(key) => plugin.on_key_down(key),
                PluginEvent::OnKeyUp(key) => plugin.on_key_up(key),
                PluginEvent::OnPluginLoad() => plugin.on_plugin_load(),
                PluginEvent::OnBeforePluginUnload() => plugin.on_before_plugin_unload()
            }
        }
    }

    pub fn read_requests(&mut self) {
        for (plugin, channel) in self.loaded.iter_mut() {
            if let Ok(req) = channel.receiver.try_recv() {
                println!("Received Request: {:?}", req);
                // match req {
                //     _ => todo!()
                // }
            }

            plugin.receive_responses();
        }
    }
}

#[derive(Debug, Clone)]
pub enum PluginEvent {
    OnFrameCreate(FrameMessenger),
    OnFrameDestroy(FrameMessenger),
    OnFrameUpdate(FrameMessenger),
    OnMouseMove(i32, i32),
    OnMouseDown(u8),
    OnMouseUp(u8),
    OnMouseScroll(f32, f32),
    OnKeyDown(u8),
    OnKeyUp(u8),
    OnPluginLoad(),
    OnBeforePluginUnload(),
}

#[derive(Debug, Clone)]
pub enum PluginRequest {
    CreateFrame(FrameOptions),
    GetFrameById(u32),
    CloseFrame(u32),
    GetMouse(),
    GetKeys(),
    PaintBuffer(Vec<u32>, Point2D<i32, UnknownUnit>, Size2D<i32, UnknownUnit>),
}

#[derive(Debug, Clone)]
pub enum PluginResponse {
    Frame(usize, FrameMessenger),
    Mouse(usize, IntPoint, u8, f32),
    Keys(usize, Vec<u8>, Vec<u8>),
    Buffer(usize, Vec<u32>),
    None(usize),
}
