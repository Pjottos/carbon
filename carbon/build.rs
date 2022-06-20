use protocol_scanner::{CodeBuilder, ProtocolParser};

use std::{env, fs};

fn main() {
    println!("cargo:rerun-if-changed=../protocols");

    let mut buf = vec![];
    let mut builder = CodeBuilder::default();

    for entry in fs::read_dir("../protocols").unwrap() {
        let entry = entry.unwrap();
        let mut parser = ProtocolParser::new(&entry.path());
        while parser.next(&mut buf) {}
        let protocol = parser.finish();
        builder.add_protocol(protocol);
    }

    let token_stream = builder.build();
    let text = format!("{}", token_stream);
    fs::write(
        env::var("OUT_DIR").unwrap() + "/protocols_generated.rs",
        text.as_bytes(),
    )
    .expect("failed to write generated code");
}
