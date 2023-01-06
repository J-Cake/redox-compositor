use euclid::{Angle, Point2D, Transform2D, UnknownUnit};
use raqote::*;

pub struct Cursor {
    pos: Point2D<i32, UnknownUnit>,
    cursor_size: i32,
}

impl Cursor {
    pub fn new() -> Self {
        Self {
            pos: Point2D::new(100, 100),
            cursor_size: 16,
        }
    }

    pub fn set_pos(&mut self, pos: Point2D<i32, UnknownUnit>) {
        self.pos = pos;
    }

    pub fn get_pos(&self) -> Point2D<i32, UnknownUnit> {
        self.pos
    }

    pub fn draw(&self, surface: &mut DrawTarget) {

        let c = Point2D::from((self.pos.x as f32 + self.cursor_size as f32 * 0.225, self.pos.y as f32 + self.cursor_size as f32 * 0.45));
        let gradient = Source::new_radial_gradient(Gradient {
            stops: vec![
                GradientStop {
                    position: 0.,
                    color: Color::new(0x40, 0, 0, 0),
                },
                GradientStop {
                    position: 1.,
                    color: Color::new(0, 0, 0, 0),
                },
            ],
        }, c, self.cursor_size as f32, Spread::Pad);

        let mut pb = PathBuilder::new();
        pb.arc(c.x, c.y, self.cursor_size as f32, 0., 2. * std::f32::consts::PI);
        let path = pb.finish();

        surface.fill(&path, &gradient, &DrawOptions::new());

        let mut path = PathBuilder::new();

        // These parameters can be tweaked to adjust the shape of the cursor
        let h = 1.125_f32; // 1.5 // the general size of the cursor
        let l = 0.17_f32; // 0.23 // the breadth of the arrow
        let g = 2.65_f32; // 1.5 // the breadth of the tail
        let t = 67.5_f32; // 67.5 // tha angle of the arrow

        {
            let size = self.cursor_size as f32;

            path.move_to(0., 0.);

            let vectors: Vec<(f32, f32)> = vec![
                (t, 0.75),
                {
                    let len = (0.75_f32.powi(2) + l.powi(2)).sqrt();
                    let angle = t + l.atan2(0.75_f32).to_degrees();
                    (angle, len)
                },
                (090.0 - g, 0.90),
                (090.0 + g, 0.90),
                {
                    let len = (0.75_f32.powi(2) + l.powi(2)).sqrt();
                    let angle = t + l.atan2(0.75_f32).to_degrees();
                    (180.0_f32 - angle, len)
                },
                (180.0_f32 - t, 0.75),
                (000.0, 0.00),
            ];

            for (angle, length) in vectors {
                let angle = angle.to_radians();
                let length = length * h;
                path.line_to(size * length * angle.cos(), size * length * angle.sin())
            }

            path.line_to(0., 0.);
        }

        let path = path.finish();

        surface.set_transform(&Transform2D::rotation(Angle::radians((270.0_f32 + t).to_radians()))
            .then_translate(self.pos.to_vector().cast()));
        surface.stroke(&path, &Source::Solid(SolidSource::from_unpremultiplied_argb(0xff, 0, 0, 0)), &StrokeStyle {
            width: 3.,
            cap: LineCap::Round,
            join: LineJoin::Round,
            miter_limit: 0.0,
            dash_array: vec![],
            dash_offset: 0.0,
        }, &DrawOptions::default());
        surface.fill(&path, &Source::Solid(SolidSource::from_unpremultiplied_argb(0xff, 0xff, 0xff, 0xff)), &DrawOptions::default());
        surface.set_transform(&Transform::identity());
    }
}

fn main() {
    let mut dt = DrawTarget::new(800, 600);
    let cursor = Cursor::new();
    dt.clear(SolidSource::from_unpremultiplied_argb(0xff, 0xa0, 0xa0, 0xa0));
    cursor.draw(&mut dt);
    dt.write_png("cursor.png")
        .unwrap();
}
