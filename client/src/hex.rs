

pub fn hex(i: &str) -> String {
    let mut str = String::new();

    for i in i.bytes() {
        let upper = (i & 0xf0) >> 4;
        let lower = i & 0x0f;

        fn nibble(lower: u8) -> char {
            match lower {
                0 => '0',
                1 => '1',
                2 => '2',
                3 => '3',
                4 => '4',
                5 => '5',
                6 => '6',
                7 => '7',
                8 => '8',
                9 => '9',
                10 => 'A',
                11 => 'B',
                12 => 'C',
                13 => 'D',
                14 => 'E',
                15 => 'F',
                _ => '\0'
            }
        }

        str.push(nibble(upper));
        str.push(nibble(lower));
    }

    return str;
}

pub fn dehex(hex: &str) -> Result<String, String> {
    let mut bytes = vec![];

    fn nibble(i: char) -> Result<u8, String> {
        match i {
            '0' => Ok(0),
            '1' => Ok(1),
            '2' => Ok(2),
            '3' => Ok(3),
            '4' => Ok(4),
            '5' => Ok(5),
            '6' => Ok(6),
            '7' => Ok(7),
            '8' => Ok(8),
            '9' => Ok(9),
            'A' => Ok(10),
            'B' => Ok(11),
            'C' => Ok(12),
            'D' => Ok(13),
            'E' => Ok(14),
            'F' => Ok(15),
            _ => Err(format!("Invalid character '{}'", i))
        }
    }

    let mut chars = hex.bytes();
    loop {
        let Some(i) = chars.next().map(|i| nibble(i as char).unwrap()) else { break; };
        let Some(j) = chars.next().map(|i| nibble(i as char).unwrap()) else { break; };

        bytes.push(((i << 4) | (j)));
    }

    unsafe { return Ok(String::from_utf8_unchecked(bytes)) }
}