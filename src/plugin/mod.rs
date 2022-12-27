use crate::frame::FrameMessenger;
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
///     * `get_mouse()`
///     * `get_key(key)`
/// 3. Painting
///     * `paint_buffer(buffer, pos, size)`
///
/// ## objects
/// * `Frame {id, title, x, y, w, h, parent() -> Frame, get_buffer() -> Buffer, send_event(Event), close()}`
/// * `Event {type, x, y, button, key, delta}`
/// * `Buffer u32[]`

pub struct PluginManager {
    loaded: Vec<Plugin>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            loaded: vec![]
        }
    }

    pub fn load(&mut self, path: &str) -> Result<(), String> {
        let mut plugin = Plugin::new(path)?;
        plugin.run().unwrap();
        self.loaded.push(plugin);
        Ok(())
    }

    pub fn event(&self, event: PluginEvent) {
        // println!("Event: {:?}", event.clone());

        for i in &self.loaded {
            match event.clone() {
                PluginEvent::OnFrameCreate(frame) => i.on_frame_create(frame),
                PluginEvent::OnFrameDestroy(frame) => i.on_frame_destroy(frame),
                PluginEvent::OnFrameUpdate(frame) => i.on_frame_update(frame),
                PluginEvent::OnMouseMove(x, y) => i.on_mouse_move(x, y),
                PluginEvent::OnMouseDown(btn) => i.on_mouse_down(btn),
                PluginEvent::OnMouseUp(btn) => i.on_mouse_up(btn),
                PluginEvent::OnMouseScroll(dx, dy) => i.on_mouse_scroll(dx, dy),
                PluginEvent::OnKeyDown(key) => i.on_key_down(key),
                PluginEvent::OnKeyUp(key) => i.on_key_up(key),
                PluginEvent::OnPluginLoad() => i.on_plugin_load(),
                PluginEvent::OnBeforePluginUnload() => i.on_before_plugin_unload(),
                _ => todo!()
            }
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
