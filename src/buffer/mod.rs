use euclid::{Point2D, Size2D, UnknownUnit};

#[derive(Debug)]
struct Buffer {
    data: Vec<u32>,
    pub size: Size2D<i32, UnknownUnit>,
}

impl Buffer {
    pub fn new(size: Size2D<i32, UnknownUnit>) -> Buffer {
        Buffer {
            data: vec![0; (size.width * size.height) as usize],
            size,
        }
    }
}
