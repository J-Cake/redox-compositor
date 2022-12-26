use crate::plugin::plugin::Plugin;

mod plugin;

/// # public plugin API
/// ## these functions can be exported as _hooks_
/// 1. Frame
///     * `on_frame_create(frame)`
///     * `on_frame_destroy(frame)`
///     * `on_frame_update(frame)`
/// 2. Mouse
///     * `on_mouse_move(mouse)`
///     * `on_mouse_down(button)`
///     * `on_mouse_up(button)`
///     * `on_mouse_scroll(delta)`
/// 3. Keyboard
///     * `on_key_down(key)`
///     * `on_key_up(key)`
/// 5. Plugin
///     * `on_plugin_load(plugin)`
///     * `on_before_plugin_unload(plugin)`
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
    loaded: Vec<Plugin>
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
}
