use std::fs::OpenOptions;
use std::io::Read;

fn main() {
    let mut input = OpenOptions::new()
        .read(true)
        .write(true)
        .open("display:input")
        .unwrap();

    loop {
        let mut buf = [0; 4096];
        let count = input.read(&mut buf).unwrap();
        let input = String::from_utf8_lossy(&buf[..count]).to_string();
        println!("Input: {}", input);
    }
}
