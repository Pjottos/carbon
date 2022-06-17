use std::fs;

fn main() {
    let mut buf = vec![];
    for entry in fs::read_dir("protocols").unwrap() {
        let entry = entry.unwrap();
        let mut parser = protocol_scanner::ProtocolParser::new(&entry.path());
        while parser.next(&mut buf) {}
        println!("{:#?}", parser.finish());
    }
}
