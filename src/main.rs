#![feature(new_uninit)]
#![feature(fn_traits)]
#![allow(warnings)]

use std::borrow::BorrowMut;
use std::cell::{RefCell, RefMut};
use std::rc::Rc;
use raqote::{DrawTarget, SolidSource};
use crate::compositor::Compositor;
use crate::plugin::PluginManager;

mod compositor;
mod display;
mod frame;
mod config;
mod bin;
mod plugin;

fn main() {
    redox_daemon::Daemon::new(move |daemon| {
        daemon.ready().expect("erika: failed to notify parent");

        let config = config::load()
            .expect("Failed to fetch config");
        let mut mgr = PluginManager::new(config.clone())
            .expect("Failed to create Plugin Manager");

        mgr.load_plugins(&config.plugins)
            .expect("Failed to load plugins");

        mgr.run();

        std::process::exit(0);
    }).expect("Failed to launch compositor");
}
