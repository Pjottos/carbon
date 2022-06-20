use protocol_scanner::{CodeBuilder, ProtocolParser};

use std::fs;

fn main() {
    let mut buf = vec![];
    let mut builder = CodeBuilder::default();

    for entry in fs::read_dir("protocols").unwrap() {
        let entry = entry.unwrap();
        let mut parser = ProtocolParser::new(&entry.path());
        while parser.next(&mut buf) {}
        let protocol = parser.finish();
        builder.add_protocol(protocol);
    }

    let token_stream = builder.build();
    println!("{}", token_stream);
}
