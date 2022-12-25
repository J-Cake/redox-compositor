use std::cell::RefCell;
use raqote::{DrawTarget, SolidSource};
use crate::compositor::Compositor;

mod compositor;
mod display;
mod frame;
mod config;
mod bin;
mod plugin;

fn main() {
    redox_daemon::Daemon::new(move |daemon| {
        daemon.ready().expect("erika: failed to notify parent");

        // let mut dt = RefCell::new(DrawTarget::new(0, 0));
        let config = config::load()
            .expect("Failed to fetch config");
        let mut ctx = Compositor::new(config.clone())
            .expect("Failed to create compositor");
        ctx.load_plugins(&config.plugins)
            .expect("Failed to load plugins");

        // clear the surface
        ctx.surface.clear(SolidSource::from_unpremultiplied_argb(0xff, 0xff, 0xff, 0xff));
        ctx.draw();

        ctx.run();

        std::process::exit(0);
    }).expect("Failed to launch compositor");
}
