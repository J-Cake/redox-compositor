use std::io::{Cursor, Read};
use std::vec::IntoIter;

use crate::{FrameFlags, FrameRequests};
use crate::frame::{FrameEvents, Input, Rect, ZIndex};

pub trait Serialise: Sized {
    fn from_bytes(byte_buffer: &[u8]) -> Result<Self, String>;
    fn to_bytes(&self) -> Result<Vec<u8>, String>;
}

fn make_array<A, T>(slice: &[T]) -> A where A: Sized + Default + AsMut<[T]>, T: Copy {
    let mut a = Default::default();
    <A as AsMut<[T]>>::as_mut(&mut a).copy_from_slice(slice);
    return a;
}

fn concat<T: Sized, K: IntoIterator<Item=T>>(mut vec: Vec<T>, values: K) -> Vec<T> {
    vec.extend(values.into_iter());
    return vec;
}

mod impls {
    use crate::frame::ZIndex;

    use super::{make_array, Serialise};

    impl Serialise for u32 {
        fn from_bytes(byte_buffer: &[u8]) -> Result<Self, String> {
            Ok(u32::from_be_bytes(make_array(&byte_buffer[0..4])))
        }

        fn to_bytes(&self) -> Result<Vec<u8>, String> {
            todo!()
        }
    }

    impl Serialise for i32 {
        fn from_bytes(byte_buffer: &[u8]) -> Result<Self, String> {
            Ok(i32::from_be_bytes(make_array(&byte_buffer[0..4])))
        }

        fn to_bytes(&self) -> Result<Vec<u8>, String> {
            todo!()
        }
    }

    impl Serialise for u64 {
        fn from_bytes(byte_buffer: &[u8]) -> Result<Self, String> {
            Ok(u64::from_be_bytes(make_array(&byte_buffer[0..4])))
        }

        fn to_bytes(&self) -> Result<Vec<u8>, String> {
            todo!()
        }
    }

    impl Serialise for u8 {
        fn from_bytes(byte_buffer: &[u8]) -> Result<Self, String> {
            match byte_buffer.get(0) {
                Some(byte) => Ok(byte.clone()),
                None => Err(format!("No byte"))
            }
        }

        fn to_bytes(&self) -> Result<Vec<u8>, String> {
            Ok(vec![self.clone()])
        }
    }

    impl Serialise for ZIndex {
        fn from_bytes(byte_buffer: &[u8]) -> Result<Self, String> {
            if let Some(index) = byte_buffer.get(0) {
                return match index {
                    0..=85 => Ok(ZIndex::Automatic),
                    86..=170 => Ok(ZIndex::Back),
                    171..=0xff => Ok(ZIndex::Front),
                    _ => Err(format!("You've broken physics. You've made a number bigger than 0xff with 8 bits"))
                };
            }

            return Err(format!("Expected one byte"));
        }

        fn to_bytes(&self) -> Result<Vec<u8>, String> {
            Ok(vec![match self {
                ZIndex::Automatic => 0,
                ZIndex::Back => 0x88,
                ZIndex::Front => 0xFF
            }])
        }
    }
}

impl Serialise for Input {
    fn from_bytes(byte_buffer: &[u8]) -> Result<Self, String> {
        todo!()
    }
    fn to_bytes(&self) -> Result<Vec<u8>, String> {
        todo!()
    }
}

impl Serialise for FrameEvents {
    fn from_bytes(byte_buffer: &[u8]) -> Result<Self, String> {
        if let Some(r#fn) = byte_buffer.get(0) {
            return match r#fn {
                0 => if byte_buffer.len() >= 17 {
                    Ok(FrameEvents::Position(Rect {
                        x: i32::from_be_bytes(make_array(&byte_buffer[1..5])),
                        y: i32::from_be_bytes(make_array(&byte_buffer[5..9])),
                        w: u32::from_be_bytes(make_array(&byte_buffer[9..13])),
                        h: u32::from_be_bytes(make_array(&byte_buffer[13..17])),
                    }))
                } else { Err(format!("Position(i32, i32, u32, u32)")) },
                1 => if byte_buffer.len() >= 2 { Ok(FrameEvents::Visible(byte_buffer[1] > 0)) } else { Err(format!("Visible(bool)")) },
                2 => match Input::from_bytes(&byte_buffer[1..]) {
                    Ok(input) => Ok(FrameEvents::Input(input)),
                    Err(err) => Err(err)
                },
                3 => Ok(FrameEvents::Redraw()),
                4 => Ok(FrameEvents::Close()),
                5 => if byte_buffer.len() >= 9 {
                    match FrameFlags::from_bits(u64::from_be_bytes(make_array(&byte_buffer[1..9]))) {
                        Some(flags) => Ok(FrameEvents::Flags(flags)),
                        None => Err(format!("Failed to parse flag list"))
                    }
                } else { Err(format!("Flags(FrameFlags)")) },
                _ => Err(format!("No variant exists for '{}'", r#fn))
            };
        }

        return Err(format!("Expected non-zero length string"));
    }

    fn to_bytes(&self) -> Result<Vec<u8>, String> {
        Ok(match self {
            FrameEvents::Position(rect) => {
                let mut vec = vec![0x00];
                vec.extend(rect.x.to_be_bytes());
                vec.extend(rect.y.to_be_bytes());
                vec.extend(rect.w.to_be_bytes());
                vec.extend(rect.h.to_be_bytes());
                vec
            },
            FrameEvents::Visible(bool) => vec![0x01, if *bool { 0x01 } else { 0x00 }],
            FrameEvents::Input(input) => match input.to_bytes() {
                Ok(input) => concat(vec![0x02], input.into_iter()),
                Err(err) => { return Err(err) }
            },
            FrameEvents::Redraw() => vec![0x03],
            FrameEvents::Close() => vec![0x04],
            FrameEvents::Flags(flags) => concat(vec![0x05], flags.bits().to_be_bytes().into_iter())
        })
    }
}

impl Serialise for FrameRequests {
    fn from_bytes(byte_buffer: &[u8]) -> Result<Self, String> {
        if let Some(r#fn) = byte_buffer.get(0) {
            return match r#fn {
                0 => if byte_buffer.len() >= 17 {
                    Ok(FrameRequests::Position(Rect {
                        x: i32::from_be_bytes(make_array(&byte_buffer[1..5])),
                        y: i32::from_be_bytes(make_array(&byte_buffer[5..9])),
                        w: u32::from_be_bytes(make_array(&byte_buffer[9..13])),
                        h: u32::from_be_bytes(make_array(&byte_buffer[13..17])),
                    }))
                } else { Err(format!("Position(i32, i32, u32, u32)")) },
                1 => if byte_buffer.len() >= 2 { Ok(FrameRequests::Fullscreen(byte_buffer[1] > 0)) } else { Err(format!("Fullscreen(bool)")) },
                2 => if byte_buffer.len() >= 9 {
                    match FrameFlags::from_bits(u64::from_be_bytes(make_array(&byte_buffer[1..9]))) {
                        Some(flags) => Ok(FrameRequests::Flags(flags)),
                        None => Err(format!("Failed to parse flag list"))
                    }
                } else { Err(format!("Flags(FrameFlags)")) },
                3 => if byte_buffer.len() >= 2 { Ok(FrameRequests::Minimise(byte_buffer[1] > 0)) } else { Err(format!("Minimise(bool)")) },
                4 => if byte_buffer.len() >= 2 { Ok(FrameRequests::ZLock(ZIndex::from_bytes(&byte_buffer[1..]).unwrap())) } else { Err(format!("Minimise(bool)")) },

                _ => Err(format!("No variant exists for '{}'", r#fn))
            };
        }

        return Err(format!("Expected non-zero length string"));
    }

    fn to_bytes(&self) -> Result<Vec<u8>, String> {
        Ok(match self {
            FrameRequests::Position(rect) => {
                let mut vec = vec![0x00];
                vec.extend(rect.x.to_be_bytes());
                vec.extend(rect.y.to_be_bytes());
                vec.extend(rect.w.to_be_bytes());
                vec.extend(rect.h.to_be_bytes());
                vec
            }
            FrameRequests::Fullscreen(fullscreen) => vec![0x01, if *fullscreen { 0x01 } else { 0x00 }],
            FrameRequests::Flags(flags) => concat(vec![0x02], flags.bits().to_be_bytes().into_iter()),
            FrameRequests::Minimise(minimise) => vec![0x03, if *minimise { 0x01 } else { 0x00 }],
            FrameRequests::ZLock(zindex) => match zindex.to_bytes() {
                Ok(bytes) => concat(vec![0x04], bytes.into_iter()),
                Err(err) => { return Err(err); }
            },
            FrameRequests::Close() => vec![0x05]
        })
    }
}