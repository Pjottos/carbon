use protocol_scanner::{emit_stubs, CodeBuilder, ProtocolParser};

use std::{env, fs};

fn main() {
    let mut buf = vec![];

    let mut args = env::args();
    args.next();
    let protocol_path = args.next().expect("first arg should be protocol path");
    let target_path = args
        .next()
        .expect("second arg should be target path or '-' for stdout");
    let gen_stubs = args.next().map_or(true, |arg| arg != "debug_codegen");

    let mut parser = ProtocolParser::new(protocol_path.as_ref());
    while parser.next(&mut buf) {}
    let protocol = parser.finish();

    let tokens = if gen_stubs {
        emit_stubs(&protocol)
    } else {
        let mut builder = CodeBuilder::default();
        builder.add_protocol(protocol);
        builder.build()
    };

    if target_path == "-" {
        println!("{}", tokens);
    } else {
        let code = format!("{}", tokens);
        fs::write(target_path, code.as_bytes()).expect("failed to write output");
    }
}
