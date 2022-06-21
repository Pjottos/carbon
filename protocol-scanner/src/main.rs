use protocol_scanner::{emit_stubs, ProtocolParser};

use std::{env, fs};

fn main() {
    let mut buf = vec![];

    let mut args = env::args();
    args.next();
    let protocol_path = args.next().expect("first arg should be protocol path");
    let target_path = args
        .next()
        .expect("second arg should be target path or '-' for stdout");

    let mut parser = ProtocolParser::new(protocol_path.as_ref());
    while parser.next(&mut buf) {}
    let protocol = parser.finish();

    let (_name, tokens) = emit_stubs(protocol);

    if target_path == "-" {
        println!("{}", tokens);
    } else {
        let code = format!("{}", tokens);
        fs::write(target_path, code.as_bytes()).expect("failed to write stubs");
    }
}
