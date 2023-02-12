use std::cell::{Ref, RefCell};
use std::collections::VecDeque;
use std::mem::MaybeUninit;
use std::rc::Rc;
use std::sync::{mpsc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

use euclid::{Box2D, Point2D, Size2D, UnknownUnit};
use raqote::IntPoint;

use crate::compositor::{Compositor, MAX_FPS};
use crate::config::Config;
// use crate::cursor::Cursor;
use crate::frame::{FrameMessenger, FrameOptions};
use crate::plugin::plugin::{MessageID, Plugin};

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
    pub response: Sender<(MessageID, PluginResponse)>,
    pub receiver: Receiver<(MessageID, PluginRequest)>,
}

pub struct PluginManager<'a, 'b> {
    loaded: Vec<(Plugin, Channel)>,
    comp: Compositor<'a, 'b>,
    // event_receiver: Receiver<PluginEvent>
    event_receiver: Rc<Mutex<VecDeque<PluginEvent>>>,
}

impl<'a, 'b> PluginManager<'a, 'b> {
    pub fn new(config: Config) -> Result<Self, String> {
        let (comp, receiver) = Compositor::<'a, 'b>::new(config.clone())
            .expect("Failed to create Compositor");

        let mut mgr = Self {
            loaded: Vec::new(),
            comp,
            event_receiver: receiver,
        };

        Ok(mgr)
    }

    pub fn load(&mut self, path: &str) -> Result<(), String> {
        let request = mpsc::channel();
        let response = mpsc::channel();

        let mut plugin = Plugin::new(path, request.0, response.1)?;
        plugin.run().unwrap();
        self.loaded.push((plugin, Channel {
            response: response.0,
            receiver: request.1,
        }));

        Ok(())
    }

    pub(crate) fn run(&mut self) {
        loop {
            let now = std::time::Instant::now();
            self.comp.tick();
            if let Some(e) = self.event_receiver.lock().unwrap().pop_back() {
                self.event(e);
            }

            self.read_requests();
            let elapsed = now.elapsed();
            if elapsed < MAX_FPS {
                thread::sleep(MAX_FPS - elapsed);
            }
        }
    }

    pub fn event(&self, event: PluginEvent) {
        for (plugin, _) in &self.loaded {
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
            if let Ok((id, req)) = channel.receiver.try_recv() {
                match req {
                    PluginRequest::CreateFrame(options) => {
                        println!("{:?}", options);
                        if let Ok(frame) = self.comp.mk_frame(options) {
                            channel.response.send((id, PluginResponse::Frame(frame.get_messenger()))).unwrap();
                        } else {
                            eprintln!("Failed to create frame");
                        }
                    },
                    PluginRequest::CloseFrame(id) => {
                        self.comp.close_frame(id).unwrap()
                    },
                    PluginRequest::GetFrameById(frame_id) => {
                        if let Some(frame) = self.comp.get_frame_by_id(frame_id) {
                            channel.response.send((id, PluginResponse::Frame(frame.get_messenger()))).unwrap();
                        } else {
                            eprintln!("Failed to get frame by id");
                        }
                    },
                    // PluginRequest::GetMouse() => {
                    //     let mouse = self.comp.get_mouse();
                    //     channel.response.send((id, PluginResponse::Mouse(mouse))).unwrap();
                    // },
                    // PluginRequest::GetKeys() => {
                    //     let keys = self.comp.get_keys();
                    //     channel.response.send((id, PluginResponse::Keys(keys))).unwrap();
                    // },
                    PluginRequest::PaintBuffer(buffer, pos, size) => {
                        self.comp.paint_buffer(buffer, Box2D::from_origin_and_size(pos, size));
                    },
                    _ => todo!()
                }
            }

            plugin.receive_responses();
        }
    }

    pub fn load_plugins(&mut self, plugins: &Vec<String>) -> Result<(), String> {
        for i in plugins {
            let Ok(plugin) = self.load(i) else {
                return Err(format!("Failed to load plugin {}", i));
            };
        }

        Ok(())
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
    GetFrameById(usize),
    CloseFrame(usize),
    GetMouse(),
    GetKeys(),
    PaintBuffer(Vec<u32>, Point2D<i32, UnknownUnit>, Size2D<i32, UnknownUnit>),
}

#[derive(Debug, Clone)]
pub enum PluginResponse {
    Frame(FrameMessenger),
    // Mouse(IntPoint, u8, (f32, f32)),
    // Keys(Vec<u8>, Vec<u8>),
    Buffer(Vec<u32>),
    None(),
}
