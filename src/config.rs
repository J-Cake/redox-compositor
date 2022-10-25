use std::collections::HashMap;
use crate::desktop::DisplayConfiguration;

pub enum Resolution {
    Explicit(u32, u32),
    Automatic,
}

pub(crate) struct DisplayConfig {
    enabled: bool,
    resolution: Resolution,
    pos: (i32, i32),
    blend_mode: Option<raqote::BlendMode>,
}

pub(crate) type Config = HashMap<String, DisplayConfig>;

pub fn load_config() -> Result<Config, String> {
    // TODO: Replace with config daemon call
    Ok(HashMap::from([(format!("display:3/activate"), DisplayConfig {
        enabled: true,
        resolution: Resolution::Automatic,
        blend_mode: None,
        pos: (0, 0),
    })]))
}