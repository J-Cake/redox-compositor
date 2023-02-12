use std::ops::Add;

use euclid::{Angle, Box2D, Point2D, Size2D, Transform2D, UnknownUnit, Vector2D};
use raqote::{Color, DrawOptions, DrawTarget, Gradient, GradientStop, Image, LineCap, LineJoin, PathBuilder, Point, SolidSource, Source, Spread, StrokeStyle, Transform};

pub struct Cursor {
    pos: Point2D<i32, UnknownUnit>,
    prev_pos: Point2D<i32, UnknownUnit>,
    cursor_size: i32,
    bounds: Box2D<i32, UnknownUnit>,
    data: Vec<u32>,
}

impl Cursor {
    pub fn new(min_x: i32, max_x: i32, min_y: i32, max_y: i32) -> Self {
        let cursor_size = 16;
        let surface = cursor(cursor_size);

        Self {
            pos: Point2D::new((max_x - min_x) / 2, (max_y - min_y) / 2),
            prev_pos: Point2D::new((max_x - min_x) / 2, (max_y - min_y) / 2),
            cursor_size,
            bounds: Box2D::new(Point2D::new(min_x, min_y), Point2D::new(max_x, max_y)),
            data: surface.into_vec(),
        }
    }

    pub fn image(&self) -> Image {
        Image {
            width: self.cursor_size + 4,
            height: self.cursor_size + 4,
            data: &self.data,
        }
    }

    pub fn set_pos(&mut self, pos: Point2D<i32, UnknownUnit>) {
        self.prev_pos = self.pos.clone();
        self.pos = pos;
    }

    pub fn get_pos(&self) -> Point2D<i32, UnknownUnit> {
        self.pos
    }
    pub fn get_prev_pos(&self) -> Point2D<i32, UnknownUnit> {
        self.prev_pos
    }

    pub fn get_size(&self) -> Size2D<i32, UnknownUnit> {
        Size2D::new(self.cursor_size + 4, self.cursor_size + 4)
    }

    pub fn get_bounding_region(&self) -> Box2D<i32, UnknownUnit> {
        Box2D::from_origin_and_size(self.get_pos(), self.get_size())
    }

    pub fn move_cursor(&mut self, x: i32, y: i32) {
        self.pos = self.pos.add(Size2D::new(x, y))
            .clamp(self.bounds.min, self.bounds.max);
    }
}

/// Generate the cursor graphic. Very much inspired by KDE Plasma's Breeze cursors.
fn cursor(cursor_size: i32) -> DrawTarget {
    let mut surface = DrawTarget::new(cursor_size + 4, cursor_size + 4);

    // let gradient = Source::new_radial_gradient(Gradient {
    //     stops: vec![
    //         GradientStop {
    //             position: 0.,
    //             color: Color::new(0x40, 0, 0, 0),
    //         },
    //         GradientStop {
    //             position: 1.,
    //             color: Color::new(0, 0, 0, 0),
    //         },
    //     ],
    // }, Point2D::new(0., 0.), cursor_size as f32, Spread::Pad);
    //
    // let mut pb = PathBuilder::new();
    // pb.arc(0., 0., cursor_size as f32, 0., 2. * std::f32::consts::PI);
    // let path = pb.finish();
    //
    // surface.fill(&path, &gradient, &DrawOptions::new());

    let mut path = PathBuilder::new();

    // These parameters can be tweaked to adjust the shape of the cursor
    let h = 1.125_f32; // 1.5 // the general size of the cursor
    let l = 0.17_f32; // 0.23 // the breadth of the arrow
    let g = 2.65_f32; // 1.5 // the breadth of the tail
    let t = 67.5_f32; // 67.5 // tha angle of the arrow

    {
        let size = cursor_size as f32;

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
        .then_translate(Vector2D::new(2., 2.)));
    surface.stroke(&path, &Source::Solid(SolidSource::from_unpremultiplied_argb(0xff, 0, 0, 0)), &StrokeStyle {
        width: 1.,
        cap: LineCap::Round,
        join: LineJoin::Round,
        miter_limit: 0.0,
        dash_array: vec![],
        dash_offset: 0.0,
    }, &DrawOptions::default());
    surface.fill(&path, &Source::Solid(SolidSource::from_unpremultiplied_argb(0xff, 0xff, 0xff, 0xff)), &DrawOptions::default());
    surface.set_transform(&Transform::identity());

    return surface;
}
