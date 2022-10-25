use raqote::{DrawOptions, SolidSource, Source};
use client::Window;

fn main() {
    let mut win = Window::new("Test Window", 720, 480)
        .unwrap();

    win.on_update(|win| {
        let ctx = win.ctx_mut();

        ctx.clear(SolidSource::from_unpremultiplied_argb(0xff, 0xff, 0xff, 0xff));

        let blue = Source::from(SolidSource::from_unpremultiplied_argb(0xff, 0x3d, 0xb5, 0xff));
        ctx.fill_rect(100., 100., 50., 50., &blue, &DrawOptions::default());

        win.fullscreen().unwrap()
    });

    win.show();
}