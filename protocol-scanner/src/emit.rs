use crate::{Protocol, ValueType};

use convert_case::{Case, Casing};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

use std::iter;

pub struct CodeBuilder {
    interface_names: Vec<String>,
    interface_versions: Vec<u32>,
    dispatch_functions_paths: Vec<Vec<TokenStream>>,
    protocols: Vec<TokenStream>,
    demarshaller_signature: TokenStream,
}

impl Default for CodeBuilder {
    fn default() -> Self {
        let demarshaller_signature = quote! {
            (object: &mut Interface, args: &[u32], state: &mut DispatchState) -> Result<(), MessageError>
        };

        Self {
            interface_names: vec![],
            interface_versions: vec![],
            dispatch_functions_paths: vec![],
            protocols: vec![],
            demarshaller_signature,
        }
    }
}

impl CodeBuilder {
    pub fn build(self) -> TokenStream {
        let Self {
            interface_names,
            interface_versions,
            dispatch_functions_paths,
            protocols,
            demarshaller_signature,
            ..
        } = self;

        assert_eq!(interface_names.len(), interface_versions.len());
        let interface_count = interface_names.len();

        let max_request_count = dispatch_functions_paths
            .iter()
            .map(Vec::len)
            .max()
            .unwrap_or(0);
        let dispatch_entries = dispatch_functions_paths.into_iter().map(|paths| {
            let funcs = paths
                .into_iter()
                .map(|path| quote! { Some(#path) })
                .chain(iter::repeat(quote! { None }))
                .take(max_request_count);

            quote! {
                [#(#funcs),*]
            }
        });

        let interface_enum_variants = interface_names.iter().map(|name| {
            let name = format_ident!("{}", name.to_case(Case::Pascal));
            quote! { #name(protocol::#name) }
        });

        quote! {
            use crate::{
                gateway::{
                    message::{MessageBuf, Write, MessageError},
                    registry::ObjectId,
                },
                protocol::{self, DispatchState},
            };
            use fixed::types::I24F8;
            use bytemuck::{cast_slice, cast_slice_mut};
            use bitflags::bitflags;

            use std::os::unix::io::RawFd;

            pub type RequestDemarshaller = fn #demarshaller_signature;
            type DispatchEntry = [Option<RequestDemarshaller>; #max_request_count];

            pub static INTERFACE_NAMES: [&str; #interface_count] = [#(#interface_names),*];
            pub static INTERFACE_VERSIONS: [u32; #interface_count] = [#(#interface_versions),*];
            pub static INTERFACE_DISPATCH_TABLE: [DispatchEntry; #interface_count] = [#(#dispatch_entries),*];

            #(#protocols)*

            pub enum Interface {
                #(#interface_enum_variants),*
            }
        }
    }

    pub fn add_protocol(&mut self, protocol: Protocol) {
        let interfaces = protocol.interfaces.iter().map(|interface| {
            let interface_mod = format_ident!("{}", interface.name);
            let interface_struct = format_ident!("{}", interface.name.to_case(Case::Pascal));

            let enums = interface.enums.iter().map(|enum_| {
                let name_str = enum_.name.to_case(Case::Pascal);
                let name = format_ident!("{}", name_str);

                let convert_variant_name = |name: &str| -> Ident {
                    let case = if enum_.is_bitfield { Case::UpperSnake } else { Case::Pascal };
                    let name = name.to_case(case);
                    match name.parse::<u32>() {
                        Ok(_) => format_ident!("U{}", name),
                        Err(_) => format_ident!("{}", name),
                    }
                };

                if enum_.is_bitfield {
                    let entries = enum_.entries.iter().map(|(name, value)| {
                        let name = convert_variant_name(name);
                        quote! { const #name = #value; }
                    });

                    quote! {
                        bitflags! {
                            #[repr(transparent)]
                            pub struct #name: u32 {
                                #(#entries)*
                            }
                        }

                        impl TryFrom<u32> for #name {
                            type Error = MessageError;

                            fn try_from(v: u32) -> Result<Self, Self::Error> {
                                #name::from_bits(v)
                                    .ok_or_else(|| MessageError::BadFormat(format!(
                                        "{:08x} is not a valid value for bitfield {}",
                                        v, #name_str,
                                    )))
                            }
                        }

                        impl From<#name> for u32 {
                            fn from(v: #name) -> u32 {
                                v.bits()
                            }
                        }
                    }
                } else {
                    let entries = enum_.entries.iter().map(|(name, value)| {
                        let name = convert_variant_name(name);
                        quote! { #name = #value }
                    });
                    let match_entries = enum_.entries.iter().map(|(name, value)| {
                        let name = convert_variant_name(name);
                        quote! { #value => Ok(Self::#name) }
                    });

                    quote! {
                        #[repr(u32)]
                        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
                        pub enum #name {
                            #(#entries),*
                        }

                        impl TryFrom<u32> for #name {
                            type Error = MessageError;

                            fn try_from(v: u32) -> Result<Self, Self::Error> {
                                match v {
                                    #(#match_entries),*,
                                    _ => Err(MessageError::BadFormat(format!(
                                        "{} is not a valid value for enum {}",
                                        v, #name_str,
                                    ))),
                                }
                            }
                        }

                        impl From<#name> for u32 {
                            fn from(v: #name) -> u32 {
                                v as u32
                            }
                        }
                    }
                }
            });

            let request_dispatches = interface.requests.iter().map(|request| {
                let extract_args = request.args.iter().map(|arg| {
                    let arg_size;
                    let cur_chunk = quote! {
                        (*args.get(__a).ok_or_else(|| MessageError::BadFormat("argument array too short".to_owned()))?)
                    };
                    let name = format_ident!("{}", &arg.name);
                    let extract = match &arg.value_type {
                        ValueType::I32 => {
                            arg_size = quote! { 1 };
                            quote! { #cur_chunk as i32 }
                        }
                        ValueType::U32 => {
                            arg_size = quote! { 1 };
                            quote! { #cur_chunk }
                        }
                        ValueType::Enum { interface, name } => {
                            arg_size = quote! { 1 };
                            let name = format_ident!("{}", name.to_case(Case::Pascal));
                            let interface = format_ident!("{}", interface);
                            let enum_ty = quote! { #interface::#name };
                            quote! { #enum_ty::try_from(#cur_chunk)? }
                        }
                        ValueType::Fixed => {
                            arg_size = quote! { 1 };
                            quote! { I24F8::from_ne_bytes(#cur_chunk.to_ne_bytes()) }
                        }
                        ValueType::ObjectId {
                            optional,
                            ..
                        } => {
                            arg_size = quote! { 1 };
                            let option = quote! { ObjectId::new(#cur_chunk) };

                            if *optional {
                                option
                            } else {
                                quote! { #option.ok_or_else(|| MessageError::BadFormat("null object id where it is not allowed".to_owned()))? }
                            }
                        }
                        ValueType::String { optional } => {
                            arg_size = quote! { 1 + (#cur_chunk as usize + 3) / 4 };
                            let create_option = quote! {
                                #cur_chunk
                                    .checked_sub(1)
                                    .map(|len| {
                                        args.get(__a + 1..).and_then(|words| {
                                            cast_slice::<_, u8>(words)
                                                .get(..len as usize)
                                                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                                        })
                                        .ok_or_else(|| MessageError::BadFormat("invalid string".to_owned()))
                                    })
                                    .transpose()?
                            };
                            if *optional {
                                create_option
                            } else {
                                quote! {
                                    #create_option.ok_or_else(|| MessageError::BadFormat("null string where it is not allowed".to_owned()))?
                                }
                            }
                        }
                        ValueType::Array { optional } => {
                            arg_size = quote! { 1 + (#cur_chunk as usize + 3) / 4 };
                            if *optional {
                                todo!("don't know how optional arrays are encoded")
                            } else {
                                quote! {
                                    args
                                        .get((__a + 1).min(args.len() - 1)..)
                                        .and_then(|words| {
                                            cast_slice::<_, u8>(words)
                                                .get(..#cur_chunk as usize)
                                        })
                                        .ok_or_else(|| MessageError::BadFormat("invalid array length".to_owned()))?
                                }
                            }
                        }
                        ValueType::Fd => {
                            arg_size = quote! { 0 };
                            quote! {
                                state.fds.pop().ok_or_else(|| MessageError::BadFormat("no fd received".to_owned()))?
                            }
                        }
                    };

                    quote! {
                        let #name = #extract;
                        __a += #arg_size;
                    }
                });

                let fn_name = format_ident!("handle_{}", &request.name);
                let demarshaller_signature = &self.demarshaller_signature;
                let args = request.args.iter().map(|arg| format_ident!("{}", arg.name));

                quote! {
                    pub fn #fn_name #demarshaller_signature {
                        let object = protocol::#interface_struct::downcast(object)
                            .expect("demarshaller called with invalid object");

                        let mut __a = 0;
                        #(#extract_args)*
                        if __a == args.len() {
                            object.#fn_name(state, #(#args),*)
                        } else {
                            Err(MessageError::BadFormat("argument array too long".to_owned()))
                        }
                    }
                }
            });

            let events = interface.events.iter().enumerate().map(|(opcode, event)| {
                let fn_name = format_ident!("emit_{}", &event.name);
                let args = event.args.iter().map(|arg| {
                    let name = format_ident!("{}", &arg.name);
                    let value_type = arg.value_type.rust_type(false);
                    quote! { #name: #value_type }
                });
                let lengths = event.args.iter().map(|arg| {
                    let name = format_ident!("{}", &arg.name);
                    match arg.value_type {
                        ValueType::I32 =>  quote! { 1 },
                        ValueType::U32 =>  quote! { 1 },
                        ValueType::Enum { .. } =>  quote! { 1 },
                        ValueType::Fixed =>  quote! { 1 },
                        ValueType::ObjectId { .. } => quote! { 1 },
                        ValueType::String { optional } => {
                            if optional {
                                quote! { 1 + #name.map_or(0, |v| (v.len() + 1 + 3) / 4) }
                            } else {
                                quote! { 1 + (#name.len() + 1 + 3) / 4 }
                            }
                        }
                        ValueType::Array { optional } => {
                            if optional {
                                quote! { 1 + #name.map_or(0, |v| (v.len() + 3) / 4) }
                            } else {
                                quote! { 1 + (#name.len() + 3) / 4 }
                            }
                        }
                        ValueType::Fd => quote! { 0 },
                    }
                });
                let mut fd_pushes = vec![];
                let write_args = event.args.iter().zip(lengths.clone()).map(|(arg, length)| {
                    let name = format_ident!("{}", &arg.name);
                    let assign = match arg.value_type {
                        ValueType::I32 => quote! { __buf[__i] = #name as u32; },
                        ValueType::U32 => quote! { __buf[__i] = #name; },
                        ValueType::Enum { .. } => quote! { __buf[__i] = #name.into(); },
                        ValueType::Fixed => quote! {
                            __buf[__i] = u32::from_ne_bytes(#name.to_ne_bytes());
                        },
                        ValueType::ObjectId { optional, .. } => {
                            if optional {
                                quote! { __buf[__i] = #name.map_or(0, |v| v.raw()); }
                            } else {
                                quote! { __buf[__i] = #name.raw(); }
                            }
                        }
                        ValueType::String { optional } => {
                            let write_str = quote! {
                                __buf[__i] = #name.len() as u32 + 1;
                                let __bytes = cast_slice_mut(&mut __buf[__i + 1..]);
                                __bytes[..#name.len()].copy_from_slice(#name.as_bytes());
                                __bytes[#name.len()] = 0;
                                // Leaking the value of the padding bytes is fine because
                                // the buffer is only used by one client.
                            };
                            if optional {
                                quote! {
                                    if let Some(#name) = #name {
                                        #write_str
                                    } else {
                                        __buf[__i] = 0;
                                    }
                                }
                            } else {
                                write_str
                            }
                        }
                        ValueType::Array { optional } => {
                            let write_array = quote! {
                                __buf[__i] = #name.len() as u32;
                                let __bytes = cast_slice_mut(&mut __buf[__i + 1..]);
                                __bytes[..#name.len()].copy_from_slice(#name);
                                // Leaking the value of the padding bytes is fine because
                                // the buffer is only used by one client.
                            };
                            if optional {
                                quote! {
                                    if let Some(#name) = #name {
                                        #write_array
                                    } else {
                                        __buf[__i] = 0;
                                    }
                                }
                            } else {
                                write_array
                            }
                        }
                        ValueType::Fd => {
                            fd_pushes.push(name);
                            TokenStream::default()
                        }
                    };

                    quote! {
                        #assign
                        __i += #length;
                    }
                });

                let opcode = u16::try_from(opcode).expect("opcode does not fit in u16");
                quote! {
                    pub fn #fn_name(
                        send_buf: &mut MessageBuf<Write>,
                        self_id: ObjectId,
                        #(#args),*
                    ) -> Result<(), MessageError> {
                        let __len = 2 #( + #lengths)*;
                        let __buf = send_buf.allocate(__len)?;
                        __buf[0] = self_id.raw();
                        // Cast is okay provided that send_buf allocation will fail
                        // for large sizes.
                        let __msg_len = __len as u16 * 4;
                        __buf[1] = u32::from(#opcode) | (u32::from(__msg_len) << 16);
                        let mut __i = 2;
                        #(#write_args)*
                        #(send_buf.push_fd(#fd_pushes)?;)*

                        Ok(())
                    }
                }
            });

            quote! {
                pub mod #interface_mod {
                    use super::*;

                    #(#enums)*
                    #(#request_dispatches)*
                    #(#events)*
                }

                impl protocol::#interface_struct {
                    #[inline]
                    fn downcast(object: &mut Interface) -> Option<&mut Self> {
                        match object {
                            Interface::#interface_struct(v) => Some(v),
                            _ => None,
                        }
                    }
                }
            }
        });

        let protocol_tokens = quote! {
            #(#interfaces)*
        };

        for interface in protocol.interfaces {
            let interface_mod = format_ident!("{}", interface.name);
            self.interface_names.push(interface.name);
            self.interface_versions.push(interface.version);
            let paths = interface
                .requests
                .into_iter()
                .map(|r| {
                    let fn_name = format_ident!("handle_{}", r.name);
                    quote! { #interface_mod::#fn_name }
                })
                .collect();
            self.dispatch_functions_paths.push(paths);
        }

        self.protocols.push(protocol_tokens);
    }
}

pub fn emit_stubs(protocol: &Protocol) -> TokenStream {
    let interfaces = protocol.interfaces.iter().map(|interface| {
        let interface_name = interface.name.to_case(Case::Pascal);
        let requests = interface.requests.iter().map(|request| {
            let args = request.args.iter().map(|arg| {
                let name = format_ident!("_{}", &arg.name);
                let value_type = arg.value_type.rust_type(true);

                quote! { #name: #value_type }
            });

            let request_name = &request.name;
            let fn_name = format_ident!("handle_{}", request.name);
            quote! {
                pub fn #fn_name(&mut self, _state: &mut DispatchState, #(#args),*) -> Result<(), MessageError> {
                    todo!("{}::{}", #interface_name, #request_name)
                }
            }
        });

        let interface_name = format_ident!("{}", interface_name);
        quote! {
            pub struct #interface_name;

            impl #interface_name {
                #(#requests)*
            }
        }
    });

    let tokens = quote! {
        use crate::{
            protocol::{generated::*, Interface, DispatchState},
            gateway::{registry::ObjectId, message::MessageError},
        };

        use std::os::unix::io::RawFd;

        #(#interfaces)*
    };
    tokens
}
