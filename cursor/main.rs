use std::fs;

pub mod cursor;
pub mod scheme;

fn main() {
    redox_daemon::Daemon::new(move |daemon| {
        daemon.ready().expect("erika: failed to notify parent");

        let mut scheme = scheme::Scheme {
            displays: vec![],
            scheme: fs::File::open(":cursor").expect("Failed to open scheme"),
            cursor: cursor::Cursor::new(0, 1024, 0, 768)
        };

        scheme.run();

        std::process::exit(0);
    }).expect("Failed to launch cursor process");
}
