use quick_xml::{events::Event, Reader};

use std::fs;

fn main() {
    let mut buf = vec![];
    for entry in fs::read_dir("protocols").unwrap() {
        let entry = entry.unwrap();
        let mut reader = Reader::from_file(entry.path()).unwrap();
        reader.trim_text(true);

        loop {
            match reader.read_event(&mut buf) {
                Ok(Event::Start(ref start)) => {
                    println!("{}", String::from_utf8_lossy(start.name()));
                }
                Ok(Event::End(ref end)) => {
                    println!("/{}", String::from_utf8_lossy(end.name()));
                }
                Ok(Event::Empty(ref start)) => {
                    println!("{} /", String::from_utf8_lossy(start.name()));
                }
                Ok(Event::Text(ref text)) => {
                    println!("{}", String::from_utf8_lossy(text));
                }
                Ok(Event::Eof) => break,
                Err(e) => panic!("Error at position {}: {}", reader.buffer_position(), e),
                _ => (),
            }

            buf.clear();
        }
    }
}
