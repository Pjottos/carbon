use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quick_xml::{
    events::{BytesEnd, BytesStart, Event},
    Reader,
};
use quote::{format_ident, quote};

use std::{fs::File, io::BufReader, path::Path};

mod emit;

pub use emit::{emit_stubs, CodeBuilder};

pub struct ProtocolParser {
    reader: Reader<BufReader<File>>,
    protocol: Option<Protocol>,
    cur_interface: Option<Interface>,
    cur_enum: Option<Enum>,
    cur_callable: Option<(Callable, bool)>,
}

impl ProtocolParser {
    pub fn new(path: &Path) -> Self {
        let mut reader = Reader::from_file(path).unwrap();
        reader.trim_text(true);

        Self {
            reader,
            protocol: None,
            cur_interface: None,
            cur_enum: None,
            cur_callable: None,
        }
    }

    pub fn next(&mut self, buf: &mut Vec<u8>) -> bool {
        let res = match self.reader.read_event(buf) {
            Ok(Event::Start(ref start)) => match self.protocol.as_mut() {
                None => {
                    self.create_protocol(start);
                    true
                }
                Some(_) => self.handle_start(start),
            },
            Ok(Event::End(ref end)) => self.handle_end(end),
            Ok(Event::Empty(ref start)) => self.handle_empty(start),
            Ok(Event::Eof) => panic!("Unexpected EOF"),
            Err(e) => panic!("Error at position {}: {}", self.reader.buffer_position(), e),
            _ => true,
        };

        buf.clear();

        res
    }

    pub fn finish(self) -> Protocol {
        self.protocol.unwrap()
    }

    fn handle_start(&mut self, start: &BytesStart) -> bool {
        if self.cur_callable.is_some() {
            match start.name() {
                b"description" => true,
                tag => panic!(
                    "unexpected start tag in callable: {}",
                    String::from_utf8_lossy(tag)
                ),
            }
        } else if self.cur_enum.is_some() {
            match start.name() {
                b"description" => true,
                tag => panic!(
                    "unexpected start tag in enum: {}",
                    String::from_utf8_lossy(tag)
                ),
            }
        } else if self.cur_interface.is_some() {
            match start.name() {
                b"description" => true,
                tag @ (b"event" | b"request") => {
                    self.create_cur_callable(start, tag == b"event");
                    true
                }
                b"enum" => {
                    self.create_cur_enum(start);
                    true
                }
                tag => panic!(
                    "unexpected start tag in interface: {}",
                    String::from_utf8_lossy(tag)
                ),
            }
        } else {
            match start.name() {
                b"copyright" => true,
                b"interface" => {
                    self.create_cur_interface(start);
                    true
                }
                tag => panic!(
                    "unexpected start tag in protocol: {}",
                    String::from_utf8_lossy(tag)
                ),
            }
        }
    }

    fn handle_end(&mut self, end: &BytesEnd) -> bool {
        assert!(
            self.protocol.is_some(),
            "found end tag before protocol start"
        );

        if self.cur_callable.is_some() {
            match end.name() {
                b"description" => true,
                tag @ (b"event" | b"request") => {
                    let (callable, is_event) = self.cur_callable.take().unwrap();
                    assert_eq!(tag == b"event", is_event, "mismatched callable end tag");
                    let interface = self.cur_interface.as_mut().unwrap();

                    if is_event {
                        interface.events.push(callable);
                    } else {
                        interface.requests.push(callable);
                    }

                    true
                }
                tag => panic!(
                    "unexpected end tag in callable: {}",
                    String::from_utf8_lossy(tag)
                ),
            }
        } else if self.cur_enum.is_some() {
            match end.name() {
                b"description" => true,
                b"enum" => {
                    self.cur_interface
                        .as_mut()
                        .unwrap()
                        .enums
                        .push(self.cur_enum.take().unwrap());
                    true
                }
                tag => panic!(
                    "unexpected end tag in enum: {}",
                    String::from_utf8_lossy(tag)
                ),
            }
        } else if self.cur_interface.is_some() {
            match end.name() {
                b"description" => true,
                b"interface" => {
                    self.protocol
                        .as_mut()
                        .unwrap()
                        .interfaces
                        .push(self.cur_interface.take().unwrap());
                    true
                }
                tag => panic!(
                    "unexpected end tag in interface: {}",
                    String::from_utf8_lossy(tag)
                ),
            }
        } else {
            match end.name() {
                b"copyright" => true,
                b"protocol" => false,
                tag => panic!(
                    "unexpected end tag in protocol: {}",
                    String::from_utf8_lossy(tag)
                ),
            }
        }
    }

    fn handle_empty(&mut self, empty: &BytesStart) -> bool {
        assert!(
            self.protocol.is_some(),
            "found empty tag before protocol start"
        );

        if let Some((cur_callable, _is_event)) = self.cur_callable.as_mut() {
            match empty.name() {
                b"arg" => {
                    let arg = Self::create_arg(
                        &self.reader,
                        &self.cur_interface.as_ref().unwrap().name,
                        empty,
                    );
                    cur_callable.args.push(arg);
                    true
                }
                b"description" => true,
                tag => panic!(
                    "unexpected empty tag in callable: {}",
                    String::from_utf8_lossy(tag)
                ),
            }
        } else if let Some(cur_enum) = self.cur_enum.as_mut() {
            match empty.name() {
                b"entry" => {
                    let entry = Self::create_enum_entry(&self.reader, empty);
                    cur_enum.entries.push(entry);
                    true
                }
                tag => panic!(
                    "unexpected empty tag in enum: {}",
                    String::from_utf8_lossy(tag)
                ),
            }
        } else if self.cur_interface.is_some() {
            let tag = empty.name();
            panic!(
                "unexpected empty tag in interface: {}",
                String::from_utf8_lossy(tag)
            );
        } else {
            let tag = empty.name();
            panic!(
                "unexpected empty tag in protocol: {}",
                String::from_utf8_lossy(tag)
            );
        }
    }

    fn create_protocol(&mut self, start: &BytesStart) {
        assert_eq!(start.name(), b"protocol", "root tag must be 'protocol'");
        let name = start
            .attributes()
            .find_map(|a| {
                let a = a.unwrap();
                (a.key == b"name").then(|| a.unescape_and_decode_value(&self.reader).unwrap())
            })
            .expect("expected 'name' attribute");

        self.protocol = Some(Protocol {
            name,
            interfaces: vec![],
        });
    }

    fn create_cur_interface(&mut self, start: &BytesStart) {
        let mut name = None;
        let mut version = None;

        for attribute in start.attributes().map(Result::unwrap) {
            match attribute.key {
                b"name" => {
                    name = Some(attribute.unescape_and_decode_value(&self.reader).unwrap());
                }
                b"version" => {
                    version = Some(String::from_utf8_lossy(&attribute.value).parse().unwrap());
                }
                key => panic!(
                    "unexpected interface attribute {}",
                    String::from_utf8_lossy(key)
                ),
            }
        }

        self.cur_interface = Some(Interface {
            name: name.expect("interface has no name"),
            version: version.expect("interface has no version"),
            events: vec![],
            requests: vec![],
            enums: vec![],
        });
    }

    fn create_cur_enum(&mut self, start: &BytesStart) {
        let name = start
            .attributes()
            .find_map(|a| {
                let a = a.unwrap();
                (a.key == b"name").then(|| a.unescape_and_decode_value(&self.reader).unwrap())
            })
            .expect("expected 'name' attribute");

        self.cur_enum = Some(Enum {
            name,
            entries: vec![],
        });
    }

    fn create_cur_callable(&mut self, start: &BytesStart, is_event: bool) {
        let mut name = None;

        for attribute in start.attributes().map(Result::unwrap) {
            match attribute.key {
                b"name" => {
                    name = Some(attribute.unescape_and_decode_value(&self.reader).unwrap());
                }
                b"type" => (),
                b"since" => (),
                key => panic!(
                    "unexpected callable attribute: {}",
                    String::from_utf8_lossy(key)
                ),
            }
        }

        let callable = Callable {
            name: name.expect("callable has no name"),
            args: vec![],
        };

        self.cur_callable = Some((callable, is_event));
    }

    fn create_arg(
        reader: &Reader<BufReader<File>>,
        interface_name: &str,
        start: &BytesStart,
    ) -> Argument {
        let mut name = None;
        let mut value_type = None;
        let mut interface = None;
        let mut enum_path = None;
        let mut optional = false;

        for attribute in start.attributes().map(Result::unwrap) {
            match attribute.key {
                b"name" => {
                    name = Some(attribute.unescape_and_decode_value(reader).unwrap());
                }
                b"type" => {
                    value_type = Some(attribute.value);
                }
                b"interface" => {
                    interface = Some(attribute.unescape_and_decode_value(reader).unwrap());
                }
                b"enum" => {
                    let full = attribute.unescape_and_decode_value(reader).unwrap();
                    if let Some((interface, name)) = full.split_once('.') {
                        enum_path = Some((interface.to_owned(), name.to_owned()));
                    } else {
                        enum_path = Some((interface_name.to_owned(), full));
                    }
                }
                b"allow-null" => {
                    optional = attribute
                        .unescape_and_decode_value(reader)
                        .unwrap()
                        .parse()
                        .unwrap();
                }
                b"summary" => (),
                key => panic!("unexpected arg attribute: {}", String::from_utf8_lossy(key)),
            }
        }

        let mut value_type =
            ValueType::parse(&value_type.expect("arg has no type"), interface, optional)
                .expect("arg has invalid type");

        if let Some((interface, name)) = enum_path {
            assert!(
                matches!(value_type, ValueType::I32 | ValueType::U32),
                "invalid enum type"
            );
            value_type = ValueType::Enum { interface, name };
        }

        Argument {
            name: name.expect("arg has no name"),
            value_type,
        }
    }

    fn create_enum_entry(reader: &Reader<BufReader<File>>, start: &BytesStart) -> (String, u32) {
        let mut name = None;
        let mut value = None;

        for attribute in start.attributes().map(Result::unwrap) {
            match attribute.key {
                b"name" => {
                    name = Some(attribute.unescape_and_decode_value(reader).unwrap());
                }
                b"value" => {
                    let text = attribute.unescape_and_decode_value(reader).unwrap();
                    value = Some(
                        text.parse()
                            .or_else(|_| u32::from_str_radix(text.trim_start_matches("0x"), 16))
                            .unwrap(),
                    );
                }
                b"summary" => (),
                b"since" => (),
                key => panic!(
                    "unexpected enum entry attribute: {}",
                    String::from_utf8_lossy(key)
                ),
            }
        }

        (
            name.expect("enum entry has no name"),
            value.expect("enum entry has no value"),
        )
    }
}

#[derive(Debug)]
pub struct Protocol {
    name: String,
    interfaces: Vec<Interface>,
}

#[derive(Debug)]
struct Interface {
    name: String,
    version: u32,
    requests: Vec<Callable>,
    events: Vec<Callable>,
    enums: Vec<Enum>,
}

#[derive(Debug)]
struct Callable {
    name: String,
    args: Vec<Argument>,
}

#[derive(Debug)]
struct Argument {
    name: String,
    value_type: ValueType,
}

#[derive(Debug)]
struct Enum {
    name: String,
    entries: Vec<(String, u32)>,
}

#[derive(Debug)]
enum ValueType {
    I32,
    U32,
    Enum {
        interface: String,
        name: String,
    },
    Fixed,
    ObjectId {
        interface: Option<String>,
        optional: bool,
    },
    String {
        optional: bool,
    },
    Array {
        optional: bool,
    },
    Fd,
}

impl ValueType {
    fn parse(
        value_type: &[u8],
        interface: Option<String>,
        optional: bool,
    ) -> Result<Self, InvalidValueType> {
        match value_type {
            b"int" if !optional => Ok(Self::I32),
            b"uint" if !optional => Ok(Self::U32),
            b"fixed" if !optional => Ok(Self::Fixed),
            b"new_id" | b"object" => Ok(Self::ObjectId {
                interface,
                optional,
            }),
            b"string" => Ok(Self::String { optional }),
            b"array" => Ok(Self::Array { optional }),
            b"fd" if !optional => Ok(Self::Fd),
            _ => Err(InvalidValueType),
        }
    }

    fn rust_type(&self, is_stub: bool) -> TokenStream {
        match self {
            ValueType::I32 => quote! { i32 },
            ValueType::U32 => quote! { u32 },
            ValueType::Enum { interface, name } => {
                let name = format_ident!("{}", name.to_case(Case::Pascal));
                let interface = format_ident!("{}", interface);
                quote! { #interface::#name }
            }
            ValueType::Fixed => quote! { I24F8 },
            ValueType::ObjectId {
                interface,
                optional,
            } => {
                let id_type = if let Some(interface) = interface.as_ref() {
                    let interface = format_ident!("{}", interface.to_case(Case::Pascal));
                    if is_stub {
                        quote! { ObjectId<#interface> }
                    } else {
                        quote! { ObjectId<protocol::#interface> }
                    }
                } else {
                    quote! { ObjectId<Interface> }
                };

                if *optional {
                    quote! { Option<#id_type> }
                } else {
                    quote! { #id_type }
                }
            }
            ValueType::String { optional } => {
                if *optional {
                    quote! { Option<&str> }
                } else {
                    quote! { &str }
                }
            }
            ValueType::Array { optional } => {
                if *optional {
                    quote! { Option<&[u8]> }
                } else {
                    quote! { &[u8] }
                }
            }
            ValueType::Fd => quote! { RawFd },
        }
    }
}

#[derive(Debug)]
struct InvalidValueType;
