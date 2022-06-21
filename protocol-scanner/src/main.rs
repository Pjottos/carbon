use protocol_scanner::{emit_stubs, CodeBuilder, Protocol, ProtocolParser};

use std::{env, fs, path::Path};

fn main() {
    let mut args = env::args();
    args.next();
    let protocol_path = args.next().expect("first arg should be protocol path");
    let target_path = args
        .next()
        .expect("second arg should be target path or '-' for stdout");
    let gen_stubs = args.next().map_or(true, |arg| arg != "debug_codegen");

    let tokens;
    if gen_stubs {
        let protocol = parse_protocol(&protocol_path);
        tokens = emit_stubs(&protocol);
    } else {
        if fs::metadata(&protocol_path).unwrap().is_dir() {
            let mut builder = CodeBuilder::default();
            for entry in fs::read_dir(&protocol_path).unwrap() {
                let entry = entry.unwrap();
                let protocol = parse_protocol(entry.path());
                builder.add_protocol(protocol);
            }
            tokens = builder.build();
        } else {
            let mut builder = CodeBuilder::default();
            let protocol = parse_protocol(&protocol_path);
            builder.add_protocol(protocol);
            tokens = builder.build();
        }
    }

    if target_path == "-" {
        println!("{}", tokens);
    } else {
        let code = format!("{}", tokens);
        fs::write(target_path, code.as_bytes()).expect("failed to write output");
    }
}

fn parse_protocol<P: AsRef<Path>>(path: P) -> Protocol {
    let mut buf = vec![];

    let mut parser = ProtocolParser::new(path.as_ref());
    while parser.next(&mut buf) {}
    parser.finish()
}
